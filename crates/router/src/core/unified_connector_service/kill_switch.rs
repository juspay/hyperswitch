//! Returns a rollout scope to the shadow connector integration when a Unified Connector Service
//! call fails more than a configurable threshold number of times.
//!
//! Failures are counted in Redis via HINCRBY on a hash key scoped to the rollout scope. The read
//! path (`is_kill_switched`) compares the counter against the threshold from the RolloutConfig.
//! When the counter exceeds the threshold, the scope falls back to shadow mode.

use std::str::FromStr;

use common_enums::{connector_enums::Connector, ConnectorIntegrationType, ExecutionMode};
use error_stack::ResultExt;
use hyperswitch_interfaces::unified_connector_service::transformers::{
    UcsKillSwitchReason, UnifiedConnectorServiceError,
};
use router_env::logger;

use crate::{
    consts,
    core::{
        errors, metrics,
        unified_connector_service::{
            build_merchant_rollout_scope, determine_connector_integration_type,
        },
    },
    routes::SessionState,
};

/// Hash field name for the failure counter.
const COUNTER_FIELD: &str = "counter";

/// Whether the counter has reached or exceeded the threshold.
fn exceeds_threshold(count: u64, threshold: u64) -> bool {
    count >= threshold
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

/// Whether the kill switch should divert this scope to shadow mode.
///
/// Checks: per-scope `kill_switch_enabled` from RolloutConfig must be true. Then reads the
/// failure counter from Redis and compares against the threshold.
///
/// Fails closed: a Redis error routes to shadow (the safe path).
pub async fn is_kill_switched(
    state: &SessionState,
    rollout_scope: &str,
    kill_switch_enabled: bool,
    kill_switch_threshold: u64,
) -> bool {
    if kill_switch_enabled {
        match read_counter(state, rollout_scope).await {
            Ok(count) => {
                let exceeded = exceeds_threshold(count, kill_switch_threshold);
                if exceeded {
                    logger::debug!(
                        rollout_scope = %rollout_scope,
                        count = count,
                        threshold = kill_switch_threshold,
                        "ucs_kill_switch: counter exceeds threshold, routing to shadow"
                    );
                }
                exceeded
            }
            // Fails closed: the scope goes to shadow when Redis cannot answer.
            Err(error) => {
                logger::error!(
                    ?error,
                    rollout_scope = %rollout_scope,
                    "ucs_kill_switch: counter unreadable, routing to shadow"
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
) -> error_stack::Result<u64, storage_impl::errors::RedisError> {
    let key: redis_interface::RedisKey = counter_key(rollout_scope).as_str().into();
    let count: u64 = state
        .store
        .get_redis_conn()?
        .get_hash_field::<Option<u64>>(&key, COUNTER_FIELD)
        .await?
        .unwrap_or(0);

    Ok(count)
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

/// A failure that qualifies to increment the counter, and the scope it targets.
struct TrippableFailure {
    rollout_scope: String,
    reason: UcsKillSwitchReason,
}

/// Records a qualifying UCS failure by incrementing its scope's counter.
///
/// Never returns an error: it runs on an already-failing path and must not fail the request.
pub async fn record_failure(
    state: &SessionState,
    context: UcsFailureContext<'_>,
    execution_mode: ExecutionMode,
    error: &UnifiedConnectorServiceError,
) {
    if let Some(failure) = trippable_failure(state, &context, execution_mode, error).await {
        record_trippable_failure(state, &failure, &context, error).await;
    }
}

/// Increments the counter and logs the failure.
async fn record_trippable_failure(
    state: &SessionState,
    failure: &TrippableFailure,
    context: &UcsFailureContext<'_>,
    error: &UnifiedConnectorServiceError,
) {
    metrics::UCS_KILL_SWITCH_FAILURE.add(
        1,
        router_env::metric_attributes!(
            ("connector", context.connector_name.to_string()),
            ("flow", context.flow_name.to_string()),
            ("reason", failure.reason.to_string())
        ),
    );

    let outcome = increment_counter(state, failure, context).await;

    logger::warn!(
        rollout_scope = %failure.rollout_scope,
        merchant_id = %context.merchant_id,
        connector = %context.connector_name,
        flow = %context.flow_name,
        payment_method = %context.payment_method,
        payment_id = %context.payment_id,
        request_id = ?state.request_id,
        reason = %failure.reason,
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
    execution_mode: ExecutionMode,
    error: &UnifiedConnectorServiceError,
) -> Option<TrippableFailure> {
    // Only the path serving merchant traffic can trip, and only a connector that has a direct
    // integration to fall back to. `&&` keeps the cheap check first.
    let scope_can_trip = matches!(execution_mode, ExecutionMode::Primary)
        && !is_ucs_only_connector(state, context.connector_name).await;

    scope_can_trip
        .then(|| error.ucs_kill_switch_reason())
        .flatten()
        .map(|reason| TrippableFailure {
            rollout_scope: build_merchant_rollout_scope(
                context.merchant_id,
                context.connector_name,
                context.flow_name,
                context.payment_method,
                context.payment_method_type,
            ),
            reason,
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
) -> IncrementOutcome {
    match write_counter(state, &failure.rollout_scope).await {
        Ok(counts) => {
            metrics::UCS_KILL_SWITCH_TRIPPED.add(
                1,
                router_env::metric_attributes!(
                    ("connector", context.connector_name.to_string()),
                    ("flow", context.flow_name.to_string()),
                    ("reason", failure.reason.to_string())
                ),
            );

            logger::info!(
                rollout_scope = %failure.rollout_scope,
                count = ?counts,
                "ucs_kill_switch: counter incremented"
            );

            IncrementOutcome::Incremented
        }
        Err(error) => {
            logger::error!(
                ?error,
                rollout_scope = %failure.rollout_scope,
                "ucs_kill_switch: could not increment counter"
            );

            IncrementOutcome::WriteFailed
        }
    }
}

/// Increments the failure counter (HINCRBY) and refreshes its TTL (EXPIRE).
/// Two separate Redis calls — not atomic, but harmless: worst case the TTL refresh
/// fails and the counter expires on its previous TTL.
async fn write_counter(
    state: &SessionState,
    rollout_scope: &str,
) -> error_stack::Result<Vec<usize>, storage_impl::errors::RedisError> {
    let conn = state.store.get_redis_conn()?;
    let key: redis_interface::RedisKey = counter_key(rollout_scope).as_str().into();

    let result = conn
        .increment_fields_in_hash(&key, &[(COUNTER_FIELD, 1)])
        .await?;

    conn.set_expiry(&key, consts::UCS_KILL_SWITCH_TTL_IN_SECONDS)
        .await
        .inspect_err(|error| {
            logger::warn!(
                ?error,
                rollout_scope = %rollout_scope,
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
    /// `None` when no `RolloutConfig` exists for this scope — falls back to trip-on-first-failure.
    pub threshold: Option<u64>,
    pub tripped: bool,
}

impl common_utils::events::ApiEventMetric for KillSwitchStatusResponse {}

/// Clears the counter, returning the scope to whatever its rollout config says.
pub async fn reset(state: SessionState, listed_key: String) -> errors::RouterResponse<()> {
    let rollout_scope = rollout_scope_in(&listed_key);

    let reply = state
        .store
        .get_redis_conn()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get a redis connection to clear the UCS kill switch")?
        .delete_key(&counter_key(rollout_scope).as_str().into())
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
    let rollout_scope = rollout_scope_in(&key_or_scope);

    let counter_value: u64 = read_counter(&state, rollout_scope)
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
    let threshold: Option<u64> = state
        .store
        .find_config_by_key_unwrap_or(
            &config_key,
            Some(consts::UCS_ROLLOUT_CONFIG_NOT_CONFIGURED.to_string()),
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
        .map(|rc| rc.kill_switch_threshold);

    let tripped = match threshold {
        Some(t) => exceeds_threshold(counter_value, t),
        None => counter_value > 0,
    };

    Ok(crate::services::ApplicationResponse::Json(
        KillSwitchStatusResponse {
            rollout_scope: rollout_scope.to_string(),
            counter: counter_value,
            threshold,
            tripped,
        },
    ))
}

#[cfg(test)]
mod tests {
    use common_enums::{PaymentMethod, PaymentMethodType};

    use super::*;

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
                Some(UcsKillSwitchReason::ConnectorOutcome),
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

    #[test]
    fn failure_reasons_have_distinct_tags() {
        let tags = [
            UcsKillSwitchReason::HyperswitchResponseUndecodable.to_string(),
            UcsKillSwitchReason::HyperswitchRequestInvalid.to_string(),
            UcsKillSwitchReason::UcsRejectedRequest.to_string(),
            UcsKillSwitchReason::UcsFlowUnsupported.to_string(),
            UcsKillSwitchReason::UcsInternalError.to_string(),
            UcsKillSwitchReason::UcsUnreachable.to_string(),
            UcsKillSwitchReason::ConnectorOutcome.to_string(),
        ];
        let unique: std::collections::HashSet<_> = tags.iter().collect();

        assert_eq!(unique.len(), tags.len());
    }
}
