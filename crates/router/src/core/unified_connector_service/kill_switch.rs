//! Kill switch that returns a rollout scope to the direct connector integration when a
//! Unified Connector Service call fails deterministically.
//!
//! Tripping unnecessarily is harmless — the scope is served by the integration it used
//! before UCS — so ambiguous outcomes resolve towards the direct path.
//!
//! The trip is runtime state and lives in redis; `ucs_rollout_config` rows are never
//! written to. Keyed on the rollout scope, so a trip targets exactly the key that enabled
//! the traffic.

use common_enums::ExecutionMode;
use error_stack::ResultExt;
use hyperswitch_interfaces::unified_connector_service::transformers::{
    UcsKillSwitchReason, UnifiedConnectorServiceError,
};
use router_env::logger;

use crate::{
    consts,
    core::{
        errors, metrics, payments::helpers::is_ucs_enabled,
        unified_connector_service::build_merchant_rollout_scope,
    },
    routes::SessionState,
};

/// Redis key holding the trip for a scope.
fn trip_key(rollout_scope: &str) -> String {
    format!("{}_{rollout_scope}", consts::UCS_KILL_SWITCH_REDIS_PREFIX)
}

/// Whether the kill switch is turned on. Cached config lookup, same as `UCS_ENABLED`.
async fn is_enabled(state: &SessionState) -> bool {
    is_ucs_enabled(state, consts::UCS_KILL_SWITCH_ENABLED).await
}

/// Whether the kill switch has tripped for this scope.
///
/// Fails closed: a redis error routes to the direct integration, since an unnecessary fallback
/// is harmless and a missed one is not. Only reached once the rollout config resolved to
/// primary, so shadow traffic never pays for the lookup.
pub async fn is_tripped(state: &SessionState, rollout_scope: &str) -> bool {
    if !is_enabled(state).await {
        return false;
    }

    let redis_conn = match state.store.get_redis_conn() {
        Ok(conn) => conn,
        Err(error) => {
            logger::error!(
                ?error,
                rollout_scope = %rollout_scope,
                "ucs_kill_switch: no redis connection, routing to the direct integration"
            );
            return true;
        }
    };

    match redis_conn
        .exists::<()>(&trip_key(rollout_scope).as_str().into())
        .await
    {
        Ok(true) => {
            logger::info!(
                rollout_scope = %rollout_scope,
                "ucs_kill_switch: scope is tripped, routing to the direct integration"
            );
            true
        }
        Ok(false) => false,
        Err(error) => {
            logger::error!(
                ?error,
                rollout_scope = %rollout_scope,
                "ucs_kill_switch: trip lookup failed, routing to the direct integration"
            );
            true
        }
    }
}

/// Classifies a UCS failure and cuts the scope over if it qualifies.
///
/// Never returns an error: this runs on an already-failing path and must not fail the request.
#[allow(clippy::too_many_arguments)]
pub async fn record_failure(
    state: &SessionState,
    merchant_id: &str,
    connector_name: &str,
    flow_name: &str,
    payment_method: common_enums::PaymentMethod,
    payment_method_type: Option<common_enums::PaymentMethodType>,
    execution_mode: ExecutionMode,
    error: &UnifiedConnectorServiceError,
) {
    // Only the path serving merchant traffic can trip.
    if !matches!(execution_mode, ExecutionMode::Primary) {
        return;
    }

    let Some(reason) = error.ucs_kill_switch_reason() else {
        return;
    };

    let rollout_scope = build_merchant_rollout_scope(
        merchant_id,
        connector_name,
        flow_name,
        payment_method,
        payment_method_type,
    );

    metrics::UCS_KILL_SWITCH_FAILURE.add(
        1,
        router_env::metric_attributes!(
            ("connector", connector_name.to_string()),
            ("flow", flow_name.to_string()),
            ("reason", reason.as_str())
        ),
    );

    // Turned off: the metric above still reports what would have been tripped.
    if !is_enabled(state).await {
        logger::warn!(
            rollout_scope = %rollout_scope,
            reason = reason.as_str(),
            ucs_error = %error,
            "ucs_kill_switch: qualifying failure observed but the kill switch is turned off"
        );
        return;
    }

    trip(state, &rollout_scope, reason, error).await;
}

/// Writes the trip. `SET NX` makes it exactly-once: concurrent failures all attempt it, one
/// wins, the rest are a no-op.
async fn trip(
    state: &SessionState,
    rollout_scope: &str,
    reason: UcsKillSwitchReason,
    error: &UnifiedConnectorServiceError,
) {
    let redis_conn = match state.store.get_redis_conn() {
        Ok(conn) => conn,
        Err(error) => {
            logger::error!(
                ?error,
                rollout_scope = %rollout_scope,
                "ucs_kill_switch: no redis connection, scope stays on UCS"
            );
            return;
        }
    };

    // Carries enough to find the originating request in the logs.
    let record = serde_json::json!({
        "reason": reason.as_str(),
        "error": error.to_string(),
        "request_id": state.request_id.as_ref().map(|id| id.to_string()),
        "tripped_at": common_utils::date_time::now_unix_timestamp(),
    })
    .to_string();

    match redis_conn
        .set_key_if_not_exists_with_expiry(
            &trip_key(rollout_scope).as_str().into(),
            record,
            Some(consts::UCS_KILL_SWITCH_TTL_IN_SECONDS),
        )
        .await
    {
        Ok(redis_interface::SetnxReply::KeySet) => {
            metrics::UCS_KILL_SWITCH_TRIPPED.add(
                1,
                router_env::metric_attributes!(("reason", reason.as_str())),
            );
            logger::error!(
                rollout_scope = %rollout_scope,
                reason = reason.as_str(),
                ucs_error = %error,
                "ucs_kill_switch: tripping the scope back to the direct integration"
            );
        }
        Ok(redis_interface::SetnxReply::KeyNotSet) => {
            logger::debug!(
                rollout_scope = %rollout_scope,
                "ucs_kill_switch: scope is already tripped"
            );
        }
        Err(error) => {
            logger::error!(
                ?error,
                rollout_scope = %rollout_scope,
                "ucs_kill_switch: failed to persist the trip, scope stays on UCS"
            );
        }
    }
}

/// Scopes currently tripped. A wrapper because `Vec<String>` has no `ApiEventMetric` impl.
#[derive(Debug, serde::Serialize)]
pub struct KillSwitchListResponse {
    pub tripped_scopes: Vec<String>,
}

impl common_utils::events::ApiEventMetric for KillSwitchListResponse {}

/// Clears the trip, returning the scope to whatever its rollout config says. Explicit
/// operator action: a tripped scope is never restored automatically.
pub async fn reset(state: SessionState, rollout_scope: String) -> errors::RouterResponse<()> {
    state
        .store
        .get_redis_conn()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get a redis connection to clear the UCS kill switch")?
        .delete_key(&trip_key(&rollout_scope).as_str().into())
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to delete the UCS kill switch trip")?;

    logger::info!(
        rollout_scope = %rollout_scope,
        "ucs_kill_switch: trip cleared via api"
    );

    Ok(crate::services::ApplicationResponse::StatusOk)
}

/// Lists every scope currently tripped.
pub async fn list_tripped_scopes(
    state: SessionState,
) -> errors::RouterResponse<KillSwitchListResponse> {
    let prefix = consts::UCS_KILL_SWITCH_REDIS_PREFIX;

    let tripped_scopes = state
        .store
        .get_redis_conn()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get a redis connection to list UCS kill switch trips")?
        .scan(&format!("{prefix}_*").as_str().into(), Some(100), None)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to scan UCS kill switch trips")?;

    Ok(crate::services::ApplicationResponse::Json(
        KillSwitchListResponse { tripped_scopes },
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
    fn failures_that_would_repeat_trip_the_scope() {
        let cases = [
            (
                UnifiedConnectorServiceError::ResponseDeserializationFailed,
                UcsKillSwitchReason::ResponseUndecodable,
            ),
            (
                UnifiedConnectorServiceError::ParsingFailed,
                UcsKillSwitchReason::ResponseUndecodable,
            ),
            (
                UnifiedConnectorServiceError::RequestEncodingFailed,
                UcsKillSwitchReason::RequestUnbuildable,
            ),
            (
                UnifiedConnectorServiceError::FailedToObtainAuthType,
                UcsKillSwitchReason::RequestUnbuildable,
            ),
            (
                UnifiedConnectorServiceError::MissingRequiredField {
                    field_name: "payment_method_data",
                },
                UcsKillSwitchReason::RequestUnbuildable,
            ),
            (
                UnifiedConnectorServiceError::NotImplemented("PSync".into()),
                UcsKillSwitchReason::NotImplemented,
            ),
        ];

        for (error, expected) in cases {
            assert_eq!(error.ucs_kill_switch_reason(), Some(expected), "{error:?}");
        }
    }

    #[test]
    fn transient_and_connector_failures_never_trip() {
        let cases = [
            // Fleet-wide: a rolling UCS deploy produces this across every scope at once, so
            // tripping on it would revert the whole migration and identify nothing.
            UnifiedConnectorServiceError::ConnectionError("dial".into()),
            // A decline would decline identically on the direct path.
            connector_error(402),
            // A connector timeout arrives as a ConnectorError carrying the synthetic 504, so it
            // must not be mistaken for UCS being broken.
            connector_error(504),
            // A connector's own server error is still the connector answering.
            connector_error(500),
        ];

        for error in cases {
            assert!(error.ucs_kill_switch_reason().is_none(), "{error:?}");
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
            UcsKillSwitchReason::ResponseUndecodable.as_str(),
            UcsKillSwitchReason::RequestUnbuildable.as_str(),
            UcsKillSwitchReason::NotImplemented.as_str(),
            UcsKillSwitchReason::FlowFailed.as_str(),
        ];
        let unique: std::collections::HashSet<_> = tags.iter().collect();

        assert_eq!(unique.len(), tags.len());
    }
}
