//! Shared Redis kill switches for primary UCS calls and shadow proxy traffic.
//! Primary UCS failures divert their rollout scope to shadow; qualifying HS proxy failures
//! divert shadow traffic to Direct. Both use atomic counters with configurable thresholds.

use std::str::FromStr;

use common_enums::{connector_enums::Connector, ConnectorIntegrationType, ExecutionMode};
use error_stack::ResultExt;
use hyperswitch_interfaces::unified_connector_service::transformers::{
    UcsKillSwitchReason, UnifiedConnectorServiceError,
};
use router_env::logger;

use crate::{
    consts,
    core::{errors, metrics, unified_connector_service::determine_connector_integration_type},
    routes::SessionState,
};

/// Hash field holding the integration-failure counter. Named `counter` rather than something
/// symmetrical with the decline field because scopes in flight already hold their count under
/// this name; renaming it would silently reset every live counter on deploy.
const COUNTER_FIELD: &str = "counter";

/// Hash field holding the connector-decline counter, in the same hash as [`COUNTER_FIELD`].
/// Separate because the two classes are measured against separate thresholds: sharing one
/// field would mean `connector_decline_threshold: N` trips on N failures of any kind rather
/// than N declines.
const DECLINE_COUNTER_FIELD: &str = "counter_connector_decline";

/// Whether the counter has reached or exceeded the threshold.
fn exceeds_threshold(count: u64, threshold: u64) -> bool {
    count >= threshold
}

impl UcsFailureClass {
    /// The hash field this class counts into. Each class has its own field so it is measured
    /// only against its own threshold.
    fn counter_field(self) -> &'static str {
        match self {
            Self::ConnectorDecline => DECLINE_COUNTER_FIELD,
            Self::IntegrationFailure => COUNTER_FIELD,
        }
    }
}

/// Redis key holding the failure counter for a scope.
fn counter_key(rollout_scope: &str) -> String {
    format!("{}_{rollout_scope}", consts::UCS_KILL_SWITCH_REDIS_PREFIX)
}

/// The rollout scope inside a redis key, or the input unchanged when it is already a scope.
///
/// `scan` hands back whole keys with the tenant prefix in front, and an operator may paste one of
/// those into the reset endpoint, so both callers need the scope back out.
fn rollout_scope_in(key_or_scope: &str) -> &str {
    let prefix = format!("{}_", consts::UCS_KILL_SWITCH_REDIS_PREFIX);

    match key_or_scope.split_once(prefix.as_str()) {
        Some((_, rollout_scope)) => rollout_scope,
        None => key_or_scope,
    }
}

/// Both of a scope's counters and whether each has reached its own threshold.
struct ScopeCounters {
    count: u64,
    exceeded: bool,
    decline_count: u64,
    decline_exceeded: bool,
}

/// Reads both counters for a scope. Each class is compared only against its own threshold;
/// a scope with no `connector_decline_threshold` never trips on declines.
async fn read_counters(
    state: &SessionState,
    rollout_scope: &str,
    settings: &RolloutSettings,
) -> error_stack::Result<ScopeCounters, storage_impl::errors::RedisError> {
    let count = read_counter(state, rollout_scope, UcsFailureClass::IntegrationFailure).await?;

    // Nothing to compare against when the scope sets no threshold, so the read is skipped
    // rather than fetched and ignored.
    let decline_count = if settings.connector_decline_threshold.is_some() {
        read_counter(state, rollout_scope, UcsFailureClass::ConnectorDecline).await?
    } else {
        0
    };

    Ok(ScopeCounters {
        count,
        exceeded: exceeds_threshold(count, settings.kill_switch_threshold),
        decline_count,
        decline_exceeded: settings
            .connector_decline_threshold
            .is_some_and(|threshold| exceeds_threshold(decline_count, threshold)),
    })
}

/// Whether the kill switch should divert this scope to shadow mode.
///
/// Checks: per-scope `kill_switch_enabled` from RolloutConfig must be true. Then reads the
/// failure counter from Redis and compares against the threshold.
///
/// Fails closed: a Redis error routes to shadow (the safe path).
pub async fn is_kill_switched(
    state: &SessionState,
    rollout_scope: &str,
    settings: RolloutSettings,
) -> bool {
    let kill_switch_enabled = settings.kill_switch_enabled;
    let kill_switch_threshold = settings.kill_switch_threshold;

    if kill_switch_enabled {
        match read_counters(state, rollout_scope, &settings).await {
            Ok(ScopeCounters {
                count,
                exceeded,
                decline_count,
                decline_exceeded,
            }) => {
                // Only when it matters. This runs on every UCS call, and the counter's
                // value is already reported by UCS_KILL_SWITCH_COUNTER_INCREMENTED at the
                // moment it changes; logging every read would re-state that at the highest
                // frequency in the module.
                // Both can be over at once, so the log names each independently rather
                // than picking one `failure_class` and hiding the other.
                if exceeded || decline_exceeded {
                    logger::warn!(
                        rollout_scope = %rollout_scope,
                        kill_switch_enabled = kill_switch_enabled,
                        redis_count = count,
                        threshold = kill_switch_threshold,
                        integration_exceeded = exceeded,
                        decline_count = decline_count,
                        connector_decline_threshold = ?settings.connector_decline_threshold,
                        decline_exceeded = decline_exceeded,
                        tripped = true,
                        request_id = ?state.request_id,
                        "UCS_KILL_SWITCH_COUNTER_EXCEEDS_THRESHOLD"
                    );
                }
                exceeded || decline_exceeded
            }
            // Fails closed: the scope goes to shadow when Redis cannot answer.
            Err(error) => {
                // Fails closed: the scope goes to shadow. Named so an alert can catch a
                // Redis outage diverting traffic, which no counter or metric would show.
                logger::error!(
                    ?error,
                    rollout_scope = %rollout_scope,
                    kill_switch_enabled = kill_switch_enabled,
                    threshold = kill_switch_threshold,
                    tripped = true,
                    request_id = ?state.request_id,
                    "UCS_KILL_SWITCH_COUNTER_UNREADABLE"
                );
                true
            }
        }
    } else {
        logger::debug!(
            rollout_scope = %rollout_scope,
            kill_switch_enabled = kill_switch_enabled,
            "ucs_kill_switch: kill switch is disabled for this scope"
        );
        false
    }
}

/// Reads the failure counter from the hash. Returns 0 if the key or field does not exist.
/// Redis connection or read errors are propagated so the caller can fail closed.
async fn read_counter(
    state: &SessionState,
    rollout_scope: &str,
    failure_class: UcsFailureClass,
) -> error_stack::Result<u64, storage_impl::errors::RedisError> {
    read_hash_counter(
        &state.store.get_redis_conn()?,
        &counter_key(rollout_scope),
        failure_class.counter_field(),
    )
    .await
}

/// Shared counter read for tenant-scoped UCS and global shadow proxy switches.
async fn read_hash_counter(
    conn: &redis_interface::RedisConnectionWithContext,
    key: &str,
    field: &str,
) -> error_stack::Result<u64, storage_impl::errors::RedisError> {
    Ok(conn
        .get_hash_field::<Option<u64>>(&key.into(), field)
        .await?
        .unwrap_or(0))
}

/// The scope's kill switch settings, resolved once by the gate and carried to the failure
/// path so a failure counts against the same threshold the gate used.
#[derive(Debug, Clone)]
pub struct RolloutSettings {
    pub execution_mode: ExecutionMode,
    pub kill_switch_enabled: bool,
    pub kill_switch_threshold: u64,
    /// `None` means connector declines never trip this scope.
    pub connector_decline_threshold: Option<u64>,
    /// The scope the gate read this config from, carried rather than rebuilt at the failure
    /// path. A payment's later calls run under a different flow (`CreateConnectorCustomer`
    /// inherits the Authorize decision without gating for itself), so rebuilding the scope
    /// from the failing call's own flow names a key nothing ever reads: it would accumulate
    /// a counter, borrow this scope's thresholds, and divert nothing.
    ///
    /// `None` on paths no gate governs, where a failure counts against nothing.
    pub rollout_scope: Option<String>,
}

impl RolloutSettings {
    /// Settings under which the kill switch can never divert this call: used where no
    /// rollout config governs it, i.e. a shadow run or a path the gate does not gate, so
    /// nothing ever reads the counter. `kill_switch_enabled: false` here states a fact.
    pub fn without_kill_switch(execution_mode: ExecutionMode) -> Self {
        Self {
            execution_mode,
            kill_switch_enabled: false,
            kill_switch_threshold: 1,
            connector_decline_threshold: None,
            rollout_scope: None,
        }
    }
}

/// What a failing UCS call was for. A struct because transposing two of six positional strings
/// would key trips under the wrong scope.
pub struct UcsFailureContext<'a> {
    pub merchant_id: &'a str,
    pub connector_name: &'a str,
    pub flow_name: &'a str,
    pub payment_id: &'a str,
    pub payment_method: common_enums::PaymentMethod,
    pub payment_method_type: Option<common_enums::PaymentMethodType>,
}

/// Which threshold a failure is measured against. A connector decline is usually the
/// issuer's verdict and identical on the direct path, so it is counted separately from
/// everything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum UcsFailureClass {
    /// The connector refused the payment: a business outcome, the same on either path.
    ConnectorDecline,
    /// Anything else: transport, integration or request-construction failure, on either
    /// side of the gRPC call. Not necessarily UCS's fault.
    IntegrationFailure,
}

/// A failure that qualifies to increment the counter, and the scope it targets.
struct TrippableFailure {
    rollout_scope: String,
    reason: UcsKillSwitchReason,
    failure_class: UcsFailureClass,
    /// The threshold this failure is measured against: `connector_decline_threshold` for a
    /// decline, `kill_switch_threshold` otherwise.
    threshold: u64,
    kill_switch_enabled: bool,
}

/// Records a connector decline: UCS answered gRPC OK with a connector 2xx, and the
/// connector still refused the payment.
///
/// Separate entry point from [`check_record_failure`] because there is no
/// `UnifiedConnectorServiceError` to classify on this path — the refusal arrives as
/// `router_data.response` being `Err`. Counts only when the scope sets
/// `connector_decline_threshold`.
pub async fn record_decline(
    state: &SessionState,
    context: UcsFailureContext<'_>,
    settings: RolloutSettings,
) {
    if let Some(failure) = trippable_failure_for_reason(
        state,
        &context,
        settings,
        UcsKillSwitchReason::ConnectorDeclined,
    )
    .await
    {
        record_trippable_failure(state, &failure, &context, None).await;
    }
}

/// Records a qualifying UCS failure by incrementing its scope's counter.
///
/// Never returns an error: it runs on an already-failing path and must not fail the request.
pub async fn check_record_failure(
    state: &SessionState,
    context: UcsFailureContext<'_>,
    settings: RolloutSettings,
    error: &UnifiedConnectorServiceError,
) {
    if let Some(failure) = trippable_failure(state, &context, settings, error).await {
        record_trippable_failure(state, &failure, &context, Some(error)).await;
    }
}

/// Increments the counter and logs the failure.
async fn record_trippable_failure(
    state: &SessionState,
    failure: &TrippableFailure,
    context: &UcsFailureContext<'_>,
    error: Option<&UnifiedConnectorServiceError>,
) {
    metrics::UCS_KILL_SWITCH_FAILURE.add(
        1,
        router_env::metric_attributes!(
            ("connector", context.connector_name.to_string()),
            ("flow", context.flow_name.to_string()),
            ("reason", failure.reason.to_string()),
            ("failure_class", failure.failure_class.to_string()),
            (
                "kill_switch_enabled",
                failure.kill_switch_enabled.to_string()
            )
        ),
    );

    let (outcome, redis_count) = increment_counter(state, failure, context).await;

    // Everything an alert needs from one line: scope, threshold, resulting counter,
    // and whether that tips the scope into shadow.
    // A counter over its threshold on a scope with the switch disabled diverts nothing,
    // so it is not "tripped". Without this an alert on tripped=true fires for scopes that
    // can never divert, which is most of them.
    let tripped = failure.kill_switch_enabled
        && redis_count.is_some_and(|count| exceeds_threshold(count, failure.threshold));

    logger::warn!(
        rollout_scope = %failure.rollout_scope,
        merchant_id = %context.merchant_id,
        connector = %context.connector_name,
        flow = %context.flow_name,
        payment_method = %context.payment_method,
        payment_method_type = ?context.payment_method_type,
        payment_id = %context.payment_id,
        request_id = ?state.request_id,
        reason = %failure.reason,
        failure_class = %failure.failure_class,
        kill_switch_enabled = failure.kill_switch_enabled,
        threshold = failure.threshold,
        redis_count = ?redis_count,
        tripped = tripped,
        outcome = %outcome,
        ucs_error = ?error,
        "UCS_KILL_SWITCH_COUNTER_INCREMENTED"
    );
}

/// What this failure would trip, or `None` when nothing can come of it: shadow traffic, a
/// UCS-only connector the gate never diverts, or a failure the classifier does not qualify.
async fn trippable_failure(
    state: &SessionState,
    context: &UcsFailureContext<'_>,
    settings: RolloutSettings,
    error: &UnifiedConnectorServiceError,
) -> Option<TrippableFailure> {
    let reason = error.ucs_kill_switch_reason()?;
    trippable_failure_for_reason(state, context, settings, reason).await
}

/// Shared core: whether this scope can trip at all, which threshold applies, and the
/// scope's config for the log line.
async fn trippable_failure_for_reason(
    state: &SessionState,
    context: &UcsFailureContext<'_>,
    settings: RolloutSettings,
    reason: UcsKillSwitchReason,
) -> Option<TrippableFailure> {
    // Only the path serving merchant traffic can trip, and only a connector that has a direct
    // integration to fall back to. `&&` keeps the cheap check first.
    let scope_can_trip = matches!(settings.execution_mode, ExecutionMode::Primary)
        && !is_ucs_only_connector(state, context.connector_name).await;

    // Only a connector 2xx carrying a refusal is a decline. A connector 4xx/5xx
    // (`ConnectorRejected`) stays on `kill_switch_threshold` as before: the status code
    // alone cannot separate a genuine decline from a request UCS built wrongly.
    let failure_class = match reason {
        UcsKillSwitchReason::ConnectorDeclined => UcsFailureClass::ConnectorDecline,
        _ => UcsFailureClass::IntegrationFailure,
    };

    // The scope and thresholds are the gate's own, carried on the request. Rebuilding the
    // scope here from this call's flow would name a different key than the gate read, and
    // re-reading the config could see a different value than the gate used.
    scope_can_trip
        .then_some(match failure_class {
            // Declines only count when the scope opts in with `connector_decline_threshold`;
            // existing configs are unaffected until updated.
            UcsFailureClass::ConnectorDecline => settings.connector_decline_threshold,
            UcsFailureClass::IntegrationFailure => Some(settings.kill_switch_threshold),
        })
        .flatten()
        .zip(settings.rollout_scope.clone())
        .map(|(threshold, rollout_scope)| TrippableFailure {
            rollout_scope,
            reason,
            failure_class,
            threshold,
            kill_switch_enabled: settings.kill_switch_enabled,
        })
}

/// A UCS-only connector has no direct integration to fall back to, so the gate never diverts one.
/// An unparseable connector name counts as having one, rather than silently dropping a scope that
/// can trip.
async fn is_ucs_only_connector(state: &SessionState, connector_name: &str) -> bool {
    match Connector::from_str(connector_name) {
        Ok(connector) => matches!(
            determine_connector_integration_type(state, connector).await,
            Ok(ConnectorIntegrationType::UcsConnector)
        ),
        Err(_) => false,
    }
}

/// What came of a counter increment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display)]
#[strum(serialize_all = "snake_case")]
enum IncrementOutcome {
    /// Counter was incremented successfully.
    Incremented,
    /// Redis refused the write.
    WriteFailed,
}

/// Increments the failure counter and refreshes its TTL.
async fn increment_counter(
    state: &SessionState,
    failure: &TrippableFailure,
    context: &UcsFailureContext<'_>,
) -> (IncrementOutcome, Option<u64>) {
    match write_counter(state, &failure.rollout_scope, failure.failure_class).await {
        Ok(counts) => {
            // HINCRBY returns the value of each field after the increment; this call
            // increments exactly one field.
            let redis_count = counts.first().and_then(|count| u64::try_from(*count).ok());

            // Only when the counter reaches the threshold on a scope that can actually
            // divert. Previously this fired on every increment, which made it a duplicate
            // of UCS_KILL_SWITCH_FAILURE and meant nothing counted actual trips.
            if failure.kill_switch_enabled
                && redis_count.is_some_and(|count| exceeds_threshold(count, failure.threshold))
            {
                metrics::UCS_KILL_SWITCH_TRIPPED.add(
                    1,
                    router_env::metric_attributes!(
                        ("connector", context.connector_name.to_string()),
                        ("flow", context.flow_name.to_string()),
                        ("reason", failure.reason.to_string()),
                        ("failure_class", failure.failure_class.to_string())
                    ),
                );
            }

            (IncrementOutcome::Incremented, redis_count)
        }
        Err(error) => {
            // The counter did not move, so this failure does not count toward the
            // threshold: the scope is silently less protected than configured.
            logger::error!(
                ?error,
                rollout_scope = %failure.rollout_scope,
                connector = %context.connector_name,
                flow = %context.flow_name,
                threshold = failure.threshold,
                "UCS_KILL_SWITCH_COUNTER_WRITE_FAILED"
            );

            (IncrementOutcome::WriteFailed, None)
        }
    }
}

/// Increments the failure counter (HINCRBY) and refreshes its TTL (EXPIRE).
/// Two separate Redis calls — not atomic, but harmless: worst case the TTL refresh
/// fails and the counter expires on its previous TTL.
async fn write_counter(
    state: &SessionState,
    rollout_scope: &str,
    failure_class: UcsFailureClass,
) -> error_stack::Result<Vec<usize>, storage_impl::errors::RedisError> {
    write_hash_counter(
        &state.store.get_redis_conn()?,
        &counter_key(rollout_scope),
        failure_class.counter_field(),
        consts::UCS_KILL_SWITCH_TTL_IN_SECONDS,
    )
    .await
}

/// Atomic HINCRBY, with a best-effort expiry refresh shared by both switches.
async fn write_hash_counter(
    conn: &redis_interface::RedisConnectionWithContext,
    key: &str,
    field: &str,
    ttl_in_seconds: i64,
) -> error_stack::Result<Vec<usize>, storage_impl::errors::RedisError> {
    let redis_key = key.into();
    let result = conn
        .increment_fields_in_hash(&redis_key, &[(field, 1)])
        .await?;
    conn.set_expiry(&redis_key, ttl_in_seconds)
        .await
        .inspect_err(|error| {
            logger::warn!(
                ?error,
                key,
                "ucs_kill_switch: failed to refresh counter TTL"
            );
        })
        .ok();
    Ok(result)
}

/// Whether this scope has a counter, its current value, and whether it exceeds the threshold.
#[derive(Debug, serde::Serialize)]
pub struct KillSwitchStatusResponse {
    pub rollout_scope: String,
    pub counter: u64,
    /// Shadow uses its configured threshold. For primary UCS, `None` means no rollout config.
    pub threshold: Option<u64>,
    /// Connector declines, counted separately from `counter` and measured against
    /// `connector_decline_threshold`.
    pub decline_counter: u64,
    /// `None` when the scope does not set one, in which case declines never trip it.
    pub connector_decline_threshold: Option<u64>,
    /// True when either counter has reached its own threshold — the same condition the gate
    /// diverts on.
    pub tripped: bool,
}

impl common_utils::events::ApiEventMetric for KillSwitchStatusResponse {}

/// Clears the counter, returning the scope to whatever its rollout config says.
pub async fn reset(state: SessionState, listed_key: String) -> errors::RouterResponse<()> {
    let rollout_scope = rollout_scope_in(&listed_key);
    let (conn, key) = if listed_key == consts::UCS_SHADOW_KILL_SWITCH_REDIS_KEY {
        // The MITM proxy and its shadow counter are shared across tenants. Read,
        // increment and reset that counter through the same global Redis namespace.
        (state.global_store.get_redis_conn(), listed_key.clone())
    } else {
        // Primary UCS counters belong to a tenant's merchant rollout scope.
        (state.store.get_redis_conn(), counter_key(rollout_scope))
    };
    let reply = conn
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get a redis connection to clear the UCS kill switch")?
        .delete_key(&key.as_str().into())
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to delete the UCS kill switch counter")?;

    match reply {
        redis_interface::DelReply::KeyDeleted => {
            logger::info!(
                rollout_scope = %rollout_scope,
                "ucs_kill_switch: counter cleared via api"
            );

            Ok(crate::services::ApplicationResponse::StatusOk)
        }
        redis_interface::DelReply::KeyNotDeleted => {
            Err(errors::ApiErrorResponse::GenericNotFoundError {
                message: format!("No UCS kill switch counter found for scope {rollout_scope}"),
            }
            .into())
        }
    }
}

/// Reads the counter so an on-call engineer can inspect it without Redis or log access.
pub async fn trip_status(
    state: SessionState,
    key_or_scope: String,
) -> errors::RouterResponse<KillSwitchStatusResponse> {
    if key_or_scope == consts::UCS_SHADOW_KILL_SWITCH_REDIS_KEY {
        let counter = shadow::read_counter(&state)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to read the shadow kill switch counter")?;
        let threshold = shadow::threshold(&state);
        return Ok(crate::services::ApplicationResponse::Json(
            KillSwitchStatusResponse {
                rollout_scope: key_or_scope,
                counter,
                threshold: Some(threshold),
                decline_counter: 0,
                connector_decline_threshold: None,
                tripped: exceeds_threshold(counter, threshold),
            },
        ));
    }

    let rollout_scope = rollout_scope_in(&key_or_scope);

    let counter_value: u64 =
        read_counter(&state, rollout_scope, UcsFailureClass::IntegrationFailure)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to read the UCS kill switch counter")?;

    // Fetch the kill_switch_threshold from the RolloutConfig for this scope.
    // Uses the scope-level config key (without org prefix) since trip_status
    // doesn't have org context.
    let config_key = format!(
        "{}_{}",
        consts::UCS_ROLLOUT_PERCENT_CONFIG_PREFIX,
        rollout_scope
    );
    let rollout_config: Option<(u64, Option<u64>)> = state
        .store
        .find_config_by_key_unwrap_or(
            &config_key,
            consts::UCS_ROLLOUT_CONFIG_NOT_CONFIGURED.to_string(),
        )
        .await
        .ok()
        .filter(|config| config.config != consts::UCS_ROLLOUT_CONFIG_NOT_CONFIGURED)
        .and_then(|config| {
            serde_json::from_str::<crate::core::payments::helpers::RolloutConfig>(&config.config)
                .inspect_err(|err| {
                    logger::warn!(
                        ?err,
                        config_key = %config_key,
                        "ucs_kill_switch: failed to parse RolloutConfig for threshold lookup"
                    );
                })
                .ok()
        })
        .map(|rc| (rc.kill_switch_threshold, rc.connector_decline_threshold));

    let (threshold, connector_decline_threshold) = match rollout_config {
        Some((integration, decline)) => (Some(integration), decline),
        None => (None, None),
    };

    // Only read when something can come of it, matching the gate.
    let decline_counter = if connector_decline_threshold.is_some() {
        read_counter(&state, rollout_scope, UcsFailureClass::ConnectorDecline)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to read the UCS kill switch decline counter")?
    } else {
        0
    };

    let tripped = match threshold {
        Some(t) => exceeds_threshold(counter_value, t),
        None => counter_value > 0,
    } || connector_decline_threshold
        .is_some_and(|threshold| exceeds_threshold(decline_counter, threshold));

    Ok(crate::services::ApplicationResponse::Json(
        KillSwitchStatusResponse {
            rollout_scope: rollout_scope.to_string(),
            counter: counter_value,
            threshold,
            decline_counter,
            connector_decline_threshold,
            tripped,
        },
    ))
}

#[cfg(test)]
mod tests {
    use common_enums::{PaymentMethod, PaymentMethodType};

    use super::*;
    // Production code carries the scope from the gate rather than rebuilding it; the tests
    // still construct scopes directly to assert on their shape.
    use crate::core::unified_connector_service::build_merchant_rollout_scope;

    /// A connector's own answer, arriving through UCS.
    fn connector_error(status_code: u16) -> UnifiedConnectorServiceError {
        UnifiedConnectorServiceError::ConnectorError(Box::new(
            hyperswitch_interfaces::unified_connector_service::transformers::ConnectorErrorInner {
                code: "card_declined".to_string(),
                message: "Card was declined".to_string(),
                status_code,
                reason: None,
                connector: "cybersource".to_string(),
                connector_transaction_id: None,
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
            },
        ))
    }

    /// One merchant and connector, so a case reads as just its payment method and flow.
    fn rollout_scope(
        payment_method: PaymentMethod,
        pmt: Option<PaymentMethodType>,
        flow: &str,
    ) -> String {
        build_merchant_rollout_scope("merchant_1", "cybersource", flow, payment_method, pmt)
    }

    #[test]
    fn failures_are_labelled_by_where_they_originated() {
        let cases = [
            (
                UnifiedConnectorServiceError::ResponseDeserializationFailed,
                UcsKillSwitchReason::HyperswitchResponseUndecodable,
            ),
            (
                UnifiedConnectorServiceError::ParsingFailed,
                UcsKillSwitchReason::HyperswitchResponseUndecodable,
            ),
            (
                UnifiedConnectorServiceError::RequestEncodingFailed,
                UcsKillSwitchReason::HyperswitchRequestInvalid,
            ),
            (
                UnifiedConnectorServiceError::FailedToObtainAuthType,
                UcsKillSwitchReason::HyperswitchRequestInvalid,
            ),
            (
                UnifiedConnectorServiceError::MissingRequiredField {
                    field_name: "payment_method_data".into(),
                },
                UcsKillSwitchReason::HyperswitchRequestInvalid,
            ),
            (
                UnifiedConnectorServiceError::NotImplemented("PSync".into()),
                UcsKillSwitchReason::UcsFlowUnsupported,
            ),
        ];

        for (error, expected) in cases {
            assert_eq!(error.ucs_kill_switch_reason(), Some(expected), "{error:?}");
        }
    }

    #[test]
    fn an_unreachable_ucs_trips() {
        assert_eq!(
            UnifiedConnectorServiceError::ConnectionError("dial".into()).ucs_kill_switch_reason(),
            Some(UcsKillSwitchReason::UcsUnreachable)
        );
    }

    #[test]
    fn connector_outcomes_trip_conservatively() {
        let cases = [
            connector_error(402),
            connector_error(504),
            connector_error(500),
        ];

        for error in cases {
            assert_eq!(
                error.ucs_kill_switch_reason(),
                Some(UcsKillSwitchReason::ConnectorRejected),
                "{error:?}"
            );
        }
    }

    #[test]
    fn scope_is_the_rollout_key_without_its_prefix() {
        assert_eq!(
            rollout_scope(PaymentMethod::Card, None, "Authorize"),
            "merchant_1_cybersource_card_Authorize"
        );
        assert_eq!(
            rollout_scope(
                PaymentMethod::Wallet,
                Some(PaymentMethodType::GooglePay),
                "Authorize"
            ),
            "merchant_1_cybersource_wallet_google_pay_Authorize"
        );
        assert_eq!(
            rollout_scope(PaymentMethod::Card, None, "Execute"),
            "merchant_1_cybersource_Execute"
        );
    }

    #[test]
    fn independently_enabled_keys_trip_independently() {
        let card = rollout_scope(PaymentMethod::Card, None, "Authorize");

        assert_ne!(
            card,
            rollout_scope(
                PaymentMethod::Wallet,
                Some(PaymentMethodType::GooglePay),
                "Authorize"
            )
        );
        assert_ne!(card, rollout_scope(PaymentMethod::Card, None, "PSync"));
        assert_ne!(
            card,
            build_merchant_rollout_scope(
                "merchant_2",
                "cybersource",
                "Authorize",
                PaymentMethod::Card,
                None
            )
        );
        assert_ne!(
            card,
            build_merchant_rollout_scope(
                "merchant_1",
                "adyen",
                "Authorize",
                PaymentMethod::Card,
                None
            )
        );
    }

    #[test]
    fn counter_key_cannot_collide_with_a_rollout_config_key() {
        let key = counter_key(&rollout_scope(PaymentMethod::Card, None, "Authorize"));

        assert!(key.starts_with(consts::UCS_KILL_SWITCH_REDIS_PREFIX));
        assert!(!key.starts_with(consts::UCS_ROLLOUT_PERCENT_CONFIG_PREFIX));
    }

    /// The whole point of a separate `connector_decline_threshold`: a decline must not be
    /// measured against a count that integration failures contributed to. Sharing one field
    /// would make `connector_decline_threshold: N` mean "N failures of any kind".
    #[test]
    fn each_failure_class_counts_into_its_own_field() {
        assert_ne!(
            UcsFailureClass::ConnectorDecline.counter_field(),
            UcsFailureClass::IntegrationFailure.counter_field()
        );
    }

    /// Scopes in flight already hold their count under `counter`. Renaming that field would
    /// silently reset every live counter on deploy, un-tripping scopes that had tripped.
    #[test]
    fn integration_failures_keep_the_pre_existing_field_name() {
        assert_eq!(
            UcsFailureClass::IntegrationFailure.counter_field(),
            "counter"
        );
    }

    #[test]
    fn failure_reasons_have_distinct_tags() {
        let tags = [
            UcsKillSwitchReason::HyperswitchResponseUndecodable.to_string(),
            UcsKillSwitchReason::HyperswitchRequestInvalid.to_string(),
            UcsKillSwitchReason::UcsRejectedRequest.to_string(),
            UcsKillSwitchReason::UcsFlowUnsupported.to_string(),
            UcsKillSwitchReason::UcsInternalError.to_string(),
            UcsKillSwitchReason::UcsUnreachable.to_string(),
            UcsKillSwitchReason::ConnectorRejected.to_string(),
        ];
        let unique: std::collections::HashSet<_> = tags.iter().collect();

        assert_eq!(unique.len(), tags.len());
    }
}

/// Disable shadow proxying after qualifying request failures reach a shared threshold.
/// The proxy is shared across tenants, so its counter uses `global_store` consistently
/// for increments, enforcement, status and reset. Primary UCS counters use tenant `store`.
pub mod shadow {
    use std::{error::Error, io::ErrorKind};

    use common_enums::ApiClientError;
    use hyperswitch_domain_models::router_data::ErrorResponse;
    use hyperswitch_interfaces::{api::gateway::GetRouterData, errors::ConnectorError};
    use router_env::logger;

    use super::{exceeds_threshold, read_hash_counter, write_hash_counter, COUNTER_FIELD};
    use crate::{
        consts::{
            UCS_SHADOW_KILL_SWITCH_REDIS_KEY as COUNTER_KEY, UCS_SHADOW_KILL_SWITCH_TTL_IN_SECONDS,
        },
        routes::SessionState,
    };

    /// Classification is separate from the explicit counter eligibility policy.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display)]
    #[strum(serialize_all = "snake_case")]
    enum ShadowFailureReason {
        RequestTimeout,
        DnsFailure,
        ConnectionRefused,
        ProxyAuthenticationRequired,
        Ignored,
    }

    impl ShadowFailureReason {
        fn should_increment_counter(self) -> bool {
            match self {
                Self::RequestTimeout
                | Self::DnsFailure
                | Self::ConnectionRefused
                | Self::ProxyAuthenticationRequired => true,
                Self::Ignored => false,
            }
        }
    }

    pub(super) async fn read_counter(
        state: &SessionState,
    ) -> error_stack::Result<u64, storage_impl::errors::RedisError> {
        read_hash_counter(
            &state.global_store.get_redis_conn()?,
            COUNTER_KEY,
            COUNTER_FIELD,
        )
        .await
    }

    pub(super) fn threshold(state: &SessionState) -> u64 {
        state
            .conf
            .comparison_service
            .as_ref()
            .map_or(1, |config| config.shadow_kill_switch_threshold)
    }

    fn request_failure_reason(error: &error_stack::Report<ConnectorError>) -> ShadowFailureReason {
        let client_timeout = error
            .downcast_ref::<ApiClientError>()
            .is_some_and(ApiClientError::is_upstream_timeout);
        let connector_timeout = error.current_context().is_connector_timeout();
        if client_timeout || connector_timeout {
            ShadowFailureReason::RequestTimeout
        } else if error.downcast_ref::<http::StatusCode>()
            == Some(&http::StatusCode::PROXY_AUTHENTICATION_REQUIRED)
        {
            ShadowFailureReason::ProxyAuthenticationRequired
        } else {
            match error.downcast_ref::<reqwest::Error>() {
                Some(error) if error.is_timeout() => ShadowFailureReason::RequestTimeout,
                Some(error) if error.is_connect() => connection_failure_reason(error),
                _ => ShadowFailureReason::Ignored,
            }
        }
    }

    fn connection_failure_reason(error: &reqwest::Error) -> ShadowFailureReason {
        std::iter::successors(error.source(), |source| (*source).source())
            .map(|cause| {
                let connection_refused = cause
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|error| error.kind() == ErrorKind::ConnectionRefused);
                // Hyper 0.14's private DNS error type exposes this fixed prefix.
                match cause.to_string().as_str() {
                    _ if connection_refused => ShadowFailureReason::ConnectionRefused,
                    "proxy authentication required" => {
                        ShadowFailureReason::ProxyAuthenticationRequired
                    }
                    message if message == "dns error" || message.starts_with("dns error:") => {
                        ShadowFailureReason::DnsFailure
                    }
                    _ => ShadowFailureReason::Ignored,
                }
            })
            .find(|reason| *reason != ShadowFailureReason::Ignored)
            .unwrap_or(ShadowFailureReason::Ignored)
    }

    fn response_failure_reason(error: &ErrorResponse) -> ShadowFailureReason {
        match error.status_code {
            407 => ShadowFailureReason::ProxyAuthenticationRequired,
            504 if error.code == hyperswitch_interfaces::consts::REQUEST_TIMEOUT_ERROR_CODE
                && error.message
                    == hyperswitch_interfaces::consts::REQUEST_TIMEOUT_ERROR_MESSAGE =>
            {
                ShadowFailureReason::RequestTimeout
            }
            _ => ShadowFailureReason::Ignored,
        }
    }

    /// Called only on the shadow path; classify the original request result and record
    /// timeouts, DNS failures, connection refused or explicit proxy authentication failures.
    /// These errors do not establish whether the proxy or upstream connector failed.
    /// Body/decoding errors and ordinary connector 5xx responses do not count.
    /// Redis commands use the application's existing configured timeout.
    pub async fn check_record_failure<F, Req, Resp, Output>(
        state: &SessionState,
        payment_id: &str,
        result: &common_utils::errors::CustomResult<Output, ConnectorError>,
    ) where
        Output: GetRouterData<F, Req, Resp>,
    {
        let reason = match result {
            Err(error) => request_failure_reason(error),
            Ok(output) => {
                let router_data = output.get_router_data();
                if router_data.connector_http_status_code == Some(407) {
                    ShadowFailureReason::ProxyAuthenticationRequired
                } else {
                    router_data
                        .response
                        .as_ref()
                        .err()
                        .map_or(ShadowFailureReason::Ignored, response_failure_reason)
                }
            }
        };
        if reason.should_increment_counter() {
            let write = async {
                let conn = state.global_store.get_redis_conn()?;
                write_hash_counter(
                    &conn,
                    COUNTER_KEY,
                    COUNTER_FIELD,
                    UCS_SHADOW_KILL_SWITCH_TTL_IN_SECONDS,
                )
                .await
            }
            .await;
            match write {
                Ok(counts) => {
                    let count = counts.first().and_then(|count| u64::try_from(*count).ok());
                    logger::warn!(
                        payment_id,
                        request_id = ?state.request_id,
                        count = ?count,
                        reason = %reason,
                        "SHADOW_KILL_SWITCH_COUNTER_INCREMENTED"
                    );

                    let configured_threshold = threshold(state);
                    // The atomic Redis increment returns the threshold value to only one
                    // request, so concurrent failures do not repeat the trip event.
                    if let Some(count) = count.filter(|count| *count == configured_threshold) {
                        logger::warn!(
                            key = COUNTER_KEY,
                            payment_id,
                            request_id = ?state.request_id,
                            count,
                            threshold = configured_threshold,
                            reason = %reason,
                            "SHADOW_KILL_SWITCH_TRIPPED"
                        );
                    }
                }
                Err(error) => logger::warn!(
                    payment_id,
                    request_id = ?state.request_id,
                    ?error,
                    "Shadow kill switch counter increment failed"
                ),
            }
        }
    }

    /// Read only for a resultant shadow path, before changing any proxy settings.
    /// An unreadable Redis counter routes shadow to Direct.
    pub async fn is_kill_switched(state: &SessionState) -> bool {
        match read_counter(state).await {
            Ok(count) => exceeds_threshold(count, threshold(state)),
            Err(error) => {
                logger::warn!(
                    ?error,
                    key = COUNTER_KEY,
                    "Shadow kill switch counter unavailable; routing to Direct"
                );
                true
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn proxy_authentication_and_request_timeout_responses_count() {
            let response = ErrorResponse {
                status_code: 407,
                ..Default::default()
            };
            assert_eq!(
                response_failure_reason(&response),
                ShadowFailureReason::ProxyAuthenticationRequired
            );
            assert!(response_failure_reason(&response).should_increment_counter());
            let response = ErrorResponse {
                status_code: 504,
                code: hyperswitch_interfaces::consts::REQUEST_TIMEOUT_ERROR_CODE.to_owned(),
                message: hyperswitch_interfaces::consts::REQUEST_TIMEOUT_ERROR_MESSAGE.to_owned(),
                ..Default::default()
            };
            assert_eq!(
                response_failure_reason(&response),
                ShadowFailureReason::RequestTimeout
            );
        }

        #[test]
        fn ordinary_upstream_5xx_responses_do_not_count() {
            for status_code in [500, 502, 503, 504] {
                let response = ErrorResponse {
                    status_code,
                    ..Default::default()
                };
                let reason = response_failure_reason(&response);
                assert_eq!(reason, ShadowFailureReason::Ignored);
                assert!(!reason.should_increment_counter());
            }
        }

        #[test]
        fn typed_request_timeouts_count_directly() {
            let error = error_stack::Report::new(ApiClientError::RequestTimeoutReceived)
                .change_context(ConnectorError::ProcessingStepFailed(None));
            assert_eq!(
                request_failure_reason(&error),
                ShadowFailureReason::RequestTimeout
            );
            let error = error_stack::Report::new(ConnectorError::RequestTimeoutReceived);
            assert_eq!(
                request_failure_reason(&error),
                ShadowFailureReason::RequestTimeout
            );
            assert!(request_failure_reason(&error).should_increment_counter());
            assert_eq!(
                request_failure_reason(&error).to_string(),
                "request_timeout"
            );
        }

        #[test]
        fn arbitrary_io_and_connection_closed_errors_do_not_count() {
            let error = error_stack::Report::new(std::io::Error::from(ErrorKind::ConnectionReset))
                .change_context(ConnectorError::ProcessingStepFailed(None));
            let reason = request_failure_reason(&error);
            assert_eq!(reason, ShadowFailureReason::Ignored);
            assert!(!reason.should_increment_counter());
            let error = error_stack::Report::new(ApiClientError::ConnectionClosedIncompleteMessage)
                .change_context(ConnectorError::ProcessingStepFailed(None));
            assert_eq!(request_failure_reason(&error), ShadowFailureReason::Ignored);
        }

        #[tokio::test]
        async fn connection_refused_counts_from_the_original_request() {
            // Reserve the port without listening, so another test cannot claim it.
            let socket = tokio::net::TcpSocket::new_v4().expect("socket");
            socket
                .bind("127.0.0.1:0".parse().expect("address"))
                .expect("bind");
            let error = reqwest::Client::builder()
                .no_proxy()
                .build()
                .expect("client")
                .get(format!("http://{}/", socket.local_addr().expect("address")))
                .send()
                .await
                .expect_err("connection refused");
            let error = error_stack::Report::new(error)
                .change_context(ConnectorError::ProcessingStepFailed(None));
            assert_eq!(
                request_failure_reason(&error),
                ShadowFailureReason::ConnectionRefused
            );
            assert!(request_failure_reason(&error).should_increment_counter());
        }

        #[tokio::test]
        async fn request_timeout_counts_without_an_endpoint_probe() {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind");
            let address = listener.local_addr().expect("address");
            let server = tokio::spawn(async move {
                let (_connection, _) = listener.accept().await.expect("accept");
                std::future::pending::<()>().await;
            });
            let error = reqwest::Client::builder()
                .no_proxy()
                .timeout(std::time::Duration::from_millis(100))
                .build()
                .expect("client")
                .get(format!("http://{address}/"))
                .send()
                .await
                .expect_err("request timeout");
            let error = error_stack::Report::new(error)
                .change_context(ConnectorError::ProcessingStepFailed(None));
            assert_eq!(
                request_failure_reason(&error),
                ShadowFailureReason::RequestTimeout
            );
            server.abort();
        }
    }
}
