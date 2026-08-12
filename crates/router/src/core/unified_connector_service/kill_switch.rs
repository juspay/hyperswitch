//! Returns a rollout scope to the direct connector integration when a Unified Connector Service
//! call fails.
//!
//! Trips live in redis, keyed on the rollout scope; `ucs_rollout_config` rows are never written
//! to. Ambiguous outcomes resolve towards the direct path, which is what the scope used before
//! UCS.

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
        payments::helpers::is_config_flag_enabled,
        unified_connector_service::{
            build_merchant_rollout_scope, determine_connector_integration_type,
        },
    },
    routes::SessionState,
};

/// Redis key holding the trip for a scope.
fn trip_key(rollout_scope: &str) -> String {
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

/// Whether the kill switch has tripped for this scope.
///
/// Fails closed: a redis error routes to the direct integration. Only reached once the rollout
/// config resolved to primary, so shadow traffic never pays for the lookup.
pub async fn is_tripped(state: &SessionState, rollout_scope: &str) -> bool {
    if is_config_flag_enabled(state, consts::UCS_KILL_SWITCH_ENABLED).await {
        match read_trip(state, rollout_scope).await {
            Ok(tripped) => tripped,
            // Fails closed: the scope goes to the direct integration when redis cannot answer.
            Err(error) => {
                logger::error!(
                    ?error,
                    rollout_scope = %rollout_scope,
                    "ucs_kill_switch: trip unreadable, routing to the direct integration"
                );
                true
            }
        }
    } else {
        false
    }
}

/// Reads the trip. Redis failures short-circuit to the caller, which decides the safe answer.
async fn read_trip(
    state: &SessionState,
    rollout_scope: &str,
) -> error_stack::Result<bool, storage_impl::errors::RedisError> {
    let tripped = state
        .store
        .get_redis_conn()?
        .exists::<()>(&trip_key(rollout_scope).as_str().into())
        .await?;

    if tripped {
        // Every request for a tripped scope reaches here; the trip is logged once elsewhere.
        logger::debug!(
            rollout_scope = %rollout_scope,
            "ucs_kill_switch: scope is tripped, routing to the direct integration"
        );
    }

    Ok(tripped)
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

/// A failure that qualifies to trip, and the scope it would trip.
struct TrippableFailure {
    rollout_scope: String,
    reason: UcsKillSwitchReason,
}

/// Records a qualifying UCS failure, and trips its scope when the switch is turned on.
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

/// Counts the failure, trips its scope when the switch is turned on, and reports what happened.
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

    let outcome = if is_config_flag_enabled(state, consts::UCS_KILL_SWITCH_ENABLED).await {
        trip(state, failure, context).await
    } else {
        TripOutcome::SwitchOff
    };

    // The one line this path emits, carrying every dimension of the decision. Alert on `outcome`
    // rather than on a message: `tripped` means a scope left UCS, `write_failed` means one should
    // have and did not. While the switch is off every line reads `switch_off`, which is the feed
    // for validating the classifier against real traffic before turning it on.
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
        "ucs_kill_switch"
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

/// What came of a trippable failure. Reported as a field on the one line the recording path
/// emits, so an alert keys on the outcome rather than on several message strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display)]
#[strum(serialize_all = "snake_case")]
enum TripOutcome {
    /// This pod wrote the trip. The scope now serves from the direct integration.
    Tripped,
    /// Another pod wrote it first.
    AlreadyTripped,
    /// The failure qualified, but enforcement is turned off.
    SwitchOff,
    /// Redis refused the write, so the scope stays on UCS.
    WriteFailed,
}

/// Writes the trip. `SET NX` makes it exactly-once: one concurrent writer wins, the rest no-op.
async fn trip(
    state: &SessionState,
    failure: &TrippableFailure,
    context: &UcsFailureContext<'_>,
) -> TripOutcome {
    let TrippableFailure {
        rollout_scope,
        reason,
    } = failure;

    let record = TripRecord {
        reason: reason.to_string(),
        request_id: state.request_id.as_ref().map(|id| id.to_string()),
        tripped_at: common_utils::date_time::now_unix_timestamp(),
    };

    match write_trip(state, rollout_scope, &record).await {
        Ok(redis_interface::SetnxReply::KeySet) => {
            metrics::UCS_KILL_SWITCH_TRIPPED.add(
                1,
                router_env::metric_attributes!(
                    ("connector", context.connector_name.to_string()),
                    ("flow", context.flow_name.to_string()),
                    ("reason", reason.to_string())
                ),
            );

            TripOutcome::Tripped
        }
        Ok(redis_interface::SetnxReply::KeyNotSet) => TripOutcome::AlreadyTripped,
        // The cause does not belong on the summary line, and it is the one thing an operator
        // cannot act on without it.
        Err(error) => {
            logger::error!(
                ?error,
                rollout_scope = %rollout_scope,
                "ucs_kill_switch: could not persist the trip"
            );

            TripOutcome::WriteFailed
        }
    }
}

/// Writes the trip if no other pod got there first. Serialisation and redis failures alike
/// short-circuit to the caller, which logs them on one path.
async fn write_trip(
    state: &SessionState,
    rollout_scope: &str,
    record: &TripRecord,
) -> error_stack::Result<redis_interface::SetnxReply, storage_impl::errors::RedisError> {
    let record = serde_json::to_string(record)
        .change_context(storage_impl::errors::RedisError::JsonSerializationFailed)?;

    state
        .store
        .get_redis_conn()?
        .set_key_if_not_exists_with_expiry(
            &trip_key(rollout_scope).as_str().into(),
            record,
            Some(consts::UCS_KILL_SWITCH_TTL_IN_SECONDS),
        )
        .await
}

/// What was written when a scope tripped. Enough to find the originating request; the error
/// itself stays out, being unbounded and unmasked connector text that the alert log already
/// carries against the same request id.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TripRecord {
    pub reason: String,
    pub request_id: Option<String>,
    pub tripped_at: i64,
}

/// Whether a scope is tripped, and what tripped it. `trip` is absent when the scope is not
/// tripped, or when its record predates this shape.
#[derive(Debug, serde::Serialize)]
pub struct KillSwitchStatusResponse {
    pub rollout_scope: String,
    pub tripped: bool,
    pub trip: Option<TripRecord>,
}

impl common_utils::events::ApiEventMetric for KillSwitchStatusResponse {}

/// Clears the trip, returning the scope to whatever its rollout config says. Takes either form
/// the list endpoint hands back.
pub async fn reset(state: SessionState, listed_key: String) -> errors::RouterResponse<()> {
    let rollout_scope = rollout_scope_in(&listed_key);

    let reply = state
        .store
        .get_redis_conn()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get a redis connection to clear the UCS kill switch")?
        .delete_key(&trip_key(rollout_scope).as_str().into())
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to delete the UCS kill switch trip")?;

    match reply {
        redis_interface::DelReply::KeyDeleted => {
            logger::info!(
                rollout_scope = %rollout_scope,
                "ucs_kill_switch: trip cleared via api"
            );

            Ok(crate::services::ApplicationResponse::StatusOk)
        }
        // Redis reports a missing key as a successful delete of nothing. Reporting that as
        // success would tell an operator a scope is back on UCS when a mistyped scope left it
        // tripped.
        redis_interface::DelReply::KeyNotDeleted => {
            Err(errors::ApiErrorResponse::GenericNotFoundError {
                message: format!("No UCS kill switch trip found for scope {rollout_scope}"),
            }
            .into())
        }
    }
}

/// Whether this scope is tripped, and what tripped it. Reads the trip record so an on-call
/// engineer has that without redis or log access. One key, so no keyspace walk.
pub async fn trip_status(
    state: SessionState,
    key_or_scope: String,
) -> errors::RouterResponse<KillSwitchStatusResponse> {
    let rollout_scope = rollout_scope_in(&key_or_scope);

    let record = state
        .store
        .get_redis_conn()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get a redis connection to read the UCS kill switch trip")?
        .get_key::<Option<String>>(&trip_key(rollout_scope).as_str().into())
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to read the UCS kill switch trip")?;

    Ok(crate::services::ApplicationResponse::Json(
        KillSwitchStatusResponse {
            rollout_scope: rollout_scope.to_string(),
            tripped: record.is_some(),
            // A record written by an older build may not parse; the scope is still tripped.
            trip: record.and_then(|record| serde_json::from_str::<TripRecord>(&record).ok()),
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
                    field_name: "payment_method_data",
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
        // While UCS is unreachable the payment has no fallback and fails outright, so serving
        // from the direct integration is strictly better.
        assert_eq!(
            UnifiedConnectorServiceError::ConnectionError("dial".into()).ucs_kill_switch_reason(),
            Some(UcsKillSwitchReason::UcsUnreachable)
        );
    }

    #[test]
    fn connector_outcomes_trip_conservatively() {
        // A false bypass to the direct path is safe — it served merchants for years.
        // A missed trip leaves merchants on a potentially broken UCS path.
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
        // A trip must target exactly the rollout key that enabled the traffic.
        assert_eq!(
            rollout_scope(PaymentMethod::Card, None, "Authorize"),
            "merchant_1_cybersource_card_Authorize"
        );
        // Wallets carry a payment method type, exactly as their rollout keys do.
        assert_eq!(
            rollout_scope(
                PaymentMethod::Wallet,
                Some(PaymentMethodType::GooglePay),
                "Authorize"
            ),
            "merchant_1_cybersource_wallet_google_pay_Authorize"
        );
        // Refund keys carry no payment method.
        assert_eq!(
            rollout_scope(PaymentMethod::Card, None, "Execute"),
            "merchant_1_cybersource_Execute"
        );
    }

    #[test]
    fn independently_enabled_keys_trip_independently() {
        // Card and wallet are enabled by separate rollout keys, and wallets fail differently,
        // so a wallet trip must not take card traffic with it.
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
    fn trip_key_cannot_collide_with_a_rollout_config_key() {
        let key = trip_key(&rollout_scope(PaymentMethod::Card, None, "Authorize"));

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
