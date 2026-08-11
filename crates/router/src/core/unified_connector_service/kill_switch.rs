//! Kill switch that returns a rollout scope to the direct connector integration when a
//! Unified Connector Service call fails deterministically.
//!
//! Cutting over unnecessarily is harmless — the scope is served by the integration it used
//! before UCS — so ambiguous outcomes resolve towards the direct path.
//!
//! The cutover is runtime state and lives in redis; `ucs_rollout_config` rows are never
//! written to. Keyed on the rollout scope, so a cutover targets exactly the key that enabled
//! the traffic.

use common_enums::ExecutionMode;
use error_stack::ResultExt;
use hyperswitch_interfaces::unified_connector_service::transformers::UnifiedConnectorServiceError;
use router_env::logger;

use crate::{
    consts,
    core::{
        errors, metrics, payments::helpers::is_ucs_enabled,
        unified_connector_service::build_rollout_scope,
    },
    routes::SessionState,
};

/// Why a UCS failure counted as a cutover trigger. Doubles as the log and metric tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UcsFailureReason {
    /// UCS answered, but the response could not be decoded into the shape Hyperswitch expects.
    ResponseUndecodable,
    /// The gRPC request could not be built from `RouterData`, or a field Hyperswitch requires was
    /// absent — the mapping between the two models is wrong for this scope.
    RequestUnbuildable,
    /// UCS reported that it does not implement this flow for this connector, so the rollout key
    /// promoting it to primary is itself wrong.
    NotImplemented,
    /// A per-flow failure marker. UCS could not complete the flow and gave no more detail.
    FlowFailed,
}

impl UcsFailureReason {
    /// Stable label used in metrics and logs.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ResponseUndecodable => "response_undecodable",
            Self::RequestUnbuildable => "request_unbuildable",
            Self::NotImplemented => "not_implemented",
            Self::FlowFailed => "flow_failed",
        }
    }
}

/// Decides whether a UCS failure should cut the scope over.
///
/// Fires on the first qualifying failure, so only failures that will repeat identically
/// qualify. Transient and availability errors are excluded: they are fleet-wide and would cut
/// over every scope at once during a rolling deploy. Connector outcomes are excluded: they
/// would fail the same way on the direct path.
///
/// Matched exhaustively so a new error variant fails to compile here rather than falling into
/// a catch-all.
pub fn classify_failure(error: &UnifiedConnectorServiceError) -> Option<UcsFailureReason> {
    use UnifiedConnectorServiceError as E;

    match error {
        // The response could not be decoded.
        E::ResponseDeserializationFailed | E::ParsingFailed => {
            Some(UcsFailureReason::ResponseUndecodable)
        }

        // The request could not be built, so the model mapping is wrong for this scope.
        E::RequestEncodingFailed
        | E::RequestEncodingFailedWithReason(_)
        | E::MissingRequiredField { .. }
        | E::MissingRequiredFields { .. }
        | E::MissingConnectorName
        | E::InvalidConnectorName
        | E::InvalidDataFormat { .. }
        | E::FailedToObtainAuthType
        | E::HeaderInjectionFailed(_) => Some(UcsFailureReason::RequestUnbuildable),

        // UCS does not implement this flow for this connector.
        E::NotImplemented(_) => Some(UcsFailureReason::NotImplemented),

        // Transport and availability: fleet-wide, so `ucs_enabled` is the lever, not this.
        E::ConnectionError(_) | E::TonicStatus { .. } => None,

        // The connector answered, including timeouts, which carry a synthetic 504. Would fail
        // the same way on the direct path.
        E::ConnectorError(_) => None,

        // Per-flow failure markers carrying no further detail. Treated as deterministic.
        E::WebhookProcessingFailure
        | E::PaymentCreateOrderFailure
        | E::PaymentAuthorizeGranularFailure
        | E::CreateSessionTokenFailure
        | E::CreateAccessTokenFailure
        | E::PaymentMethodTokenizeFailure
        | E::CreateConnectorCustomerFailure
        | E::PaymentAuthorizeFailure
        | E::PaymentPreAuthenticateFailure
        | E::PaymentAuthenticateFailure
        | E::PaymentPostAuthenticateFailure
        | E::PaymentGetFailure
        | E::PaymentCaptureFailure
        | E::PaymentSetupRecurringFailure
        | E::RecurringPaymentChargeFailure
        | E::PaymentRefundFailure
        | E::RefundSyncFailure
        | E::IncomingWebhookHandleEventFailure
        | E::IncomingWebhookParseEventFailure
        | E::PaymentVoidFailure
        | E::CreateSdkSessionTokenFailure
        | E::PaymentIncrementalAuthorizationFailure
        | E::PayoutCreateFailure
        | E::PayoutTransferFailure
        | E::PayoutGetFailure
        | E::PayoutVoidFailure
        | E::PayoutStageFailure
        | E::PayoutCreateRecipientFailure
        | E::PayoutEnrollDisburseAccountFailure
        | E::SurchargeCalculateFailure
        | E::NotifyConnectorFailure => Some(UcsFailureReason::FlowFailed),
    }
}

/// Scope a cutover covers: the rollout scope of the key that enabled the traffic.
///
/// Built by [`build_rollout_scope`] so the recording site and the enforcement site cannot
/// derive different keys.
pub fn build_scope(
    merchant_id: &str,
    connector_name: &str,
    flow_name: &str,
    payment_method: common_enums::PaymentMethod,
    payment_method_type: Option<common_enums::PaymentMethodType>,
) -> String {
    build_rollout_scope(
        merchant_id,
        connector_name,
        flow_name,
        payment_method,
        payment_method_type,
    )
}

/// Redis key holding the cutover for a scope.
fn cutover_key(scope: &str) -> String {
    format!("{}_{scope}", consts::UCS_KILL_SWITCH_REDIS_PREFIX)
}

/// Whether the kill switch is turned on. Cached config lookup, same as `UCS_ENABLED`.
async fn is_enabled(state: &SessionState) -> bool {
    is_ucs_enabled(state, consts::UCS_KILL_SWITCH_ENABLED).await
}

/// Whether UCS has been cut off for this scope.
///
/// Fails closed: a redis error routes to the direct integration, since an unnecessary fallback
/// is harmless and a missed one is not. Only reached once the rollout config resolved to
/// primary, so shadow traffic never pays for the lookup.
pub async fn is_cut_over(state: &SessionState, scope: &str) -> bool {
    if !is_enabled(state).await {
        return false;
    }

    let redis_conn = match state.store.get_redis_conn() {
        Ok(conn) => conn,
        Err(error) => {
            logger::error!(
                ?error,
                ucs_kill_switch_scope = %scope,
                "ucs_kill_switch: no redis connection, routing to the direct integration"
            );
            return true;
        }
    };

    match redis_conn
        .exists::<()>(&cutover_key(scope).as_str().into())
        .await
    {
        Ok(true) => {
            logger::info!(
                ucs_kill_switch_scope = %scope,
                "ucs_kill_switch: scope is cut over, routing to the direct integration"
            );
            true
        }
        Ok(false) => false,
        Err(error) => {
            logger::error!(
                ?error,
                ucs_kill_switch_scope = %scope,
                "ucs_kill_switch: cutover lookup failed, routing to the direct integration"
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
    // Only the path serving merchant traffic can cut over.
    if !matches!(execution_mode, ExecutionMode::Primary) {
        return;
    }

    let Some(reason) = classify_failure(error) else {
        return;
    };

    let scope = build_scope(
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

    // Turned off: the metric above still reports what would have been cut over.
    if !is_enabled(state).await {
        logger::warn!(
            ucs_kill_switch_scope = %scope,
            reason = reason.as_str(),
            "ucs_kill_switch: qualifying failure observed but the kill switch is turned off"
        );
        return;
    }

    cut_over(state, &scope, reason, error).await;
}

/// Writes the cutover. `SET NX` makes it exactly-once: concurrent failures all attempt it, one
/// wins, the rest are a no-op.
async fn cut_over(
    state: &SessionState,
    scope: &str,
    reason: UcsFailureReason,
    error: &UnifiedConnectorServiceError,
) {
    let redis_conn = match state.store.get_redis_conn() {
        Ok(conn) => conn,
        Err(error) => {
            logger::error!(
                ?error,
                ucs_kill_switch_scope = %scope,
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
        "cut_over_at": common_utils::date_time::now_unix_timestamp(),
    })
    .to_string();

    match redis_conn
        .set_key_if_not_exists_with_expiry(
            &cutover_key(scope).as_str().into(),
            record,
            Some(consts::UCS_KILL_SWITCH_TTL_IN_SECONDS),
        )
        .await
    {
        Ok(redis_interface::SetnxReply::KeySet) => {
            metrics::UCS_KILL_SWITCH_CUT_OVER.add(
                1,
                router_env::metric_attributes!(("reason", reason.as_str())),
            );
            logger::error!(
                ucs_kill_switch_scope = %scope,
                reason = reason.as_str(),
                ucs_error = %error,
                "ucs_kill_switch: cutting the scope over to the direct integration"
            );
        }
        Ok(redis_interface::SetnxReply::KeyNotSet) => {
            logger::debug!(
                ucs_kill_switch_scope = %scope,
                "ucs_kill_switch: scope is already cut over"
            );
        }
        Err(error) => {
            logger::error!(
                ?error,
                ucs_kill_switch_scope = %scope,
                "ucs_kill_switch: failed to persist the cutover, scope stays on UCS"
            );
        }
    }
}

/// Scopes currently cut over. A wrapper because `Vec<String>` has no `ApiEventMetric` impl.
#[derive(Debug, serde::Serialize)]
pub struct KillSwitchListResponse {
    pub cut_over_scopes: Vec<String>,
}

impl common_utils::events::ApiEventMetric for KillSwitchListResponse {}

/// Clears the cutover, returning the scope to whatever its rollout config says. Explicit
/// operator action: a cut-over scope is never restored automatically.
pub async fn reset_cut_over(state: SessionState, scope: String) -> errors::RouterResponse<()> {
    state
        .store
        .get_redis_conn()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get a redis connection to clear the UCS kill switch")?
        .delete_key(&cutover_key(&scope).as_str().into())
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to delete the UCS kill switch cutover")?;

    logger::info!(
        ucs_kill_switch_scope = %scope,
        "ucs_kill_switch: cutover cleared via api"
    );

    Ok(crate::services::ApplicationResponse::StatusOk)
}

/// Lists every scope currently cut over.
pub async fn list_cut_over_scopes(
    state: SessionState,
) -> errors::RouterResponse<KillSwitchListResponse> {
    let prefix = consts::UCS_KILL_SWITCH_REDIS_PREFIX;

    let cut_over_scopes = state
        .store
        .get_redis_conn()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get a redis connection to list UCS kill switch cutovers")?
        .scan(&format!("{prefix}_*").as_str().into(), Some(100), None)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to scan UCS kill switch cutovers")?;

    Ok(crate::services::ApplicationResponse::Json(
        KillSwitchListResponse { cut_over_scopes },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn transport_failures_do_not_cut_over() {
        // A rolling UCS deploy produces these across every scope at once. Cutting over on them
        // would revert the whole migration and identify nothing. `TonicStatus` shares this match
        // arm, so it is covered by the same guarantee.
        assert!(
            classify_failure(&UnifiedConnectorServiceError::ConnectionError(
                "dial".into()
            ))
            .is_none()
        );
    }

    #[test]
    fn connector_outcomes_do_not_cut_over() {
        // A decline would decline identically on the direct path.
        assert!(classify_failure(&connector_error(402)).is_none());
        // A connector timeout arrives as a ConnectorError carrying the synthetic 504, so it must
        // not be mistaken for UCS being broken.
        assert!(classify_failure(&connector_error(504)).is_none());
        // Nor should a connector's own server error.
        assert!(classify_failure(&connector_error(500)).is_none());
    }

    #[test]
    fn deterministic_failures_cut_over() {
        // These fail identically on every retry, so the first occurrence is fully informative
        // and there is nothing a threshold could add.
        assert_eq!(
            classify_failure(&UnifiedConnectorServiceError::ResponseDeserializationFailed),
            Some(UcsFailureReason::ResponseUndecodable)
        );
        assert_eq!(
            classify_failure(&UnifiedConnectorServiceError::ParsingFailed),
            Some(UcsFailureReason::ResponseUndecodable)
        );
        assert_eq!(
            classify_failure(&UnifiedConnectorServiceError::RequestEncodingFailed),
            Some(UcsFailureReason::RequestUnbuildable)
        );
        assert_eq!(
            classify_failure(&UnifiedConnectorServiceError::FailedToObtainAuthType),
            Some(UcsFailureReason::RequestUnbuildable)
        );
        assert_eq!(
            classify_failure(&UnifiedConnectorServiceError::MissingRequiredField {
                field_name: "payment_method_data"
            }),
            Some(UcsFailureReason::RequestUnbuildable)
        );
        assert_eq!(
            classify_failure(&UnifiedConnectorServiceError::NotImplemented(
                "PSync".into()
            )),
            Some(UcsFailureReason::NotImplemented)
        );
    }

    #[test]
    fn scope_matches_the_rollout_key_shape() {
        // A cutover must target exactly the rollout key that enabled the traffic, so the scope
        // is the rollout key without its prefix.
        assert_eq!(
            build_scope(
                "merchant_1",
                "cybersource",
                "Authorize",
                common_enums::PaymentMethod::Card,
                None
            ),
            "merchant_1_cybersource_card_Authorize"
        );
        // Wallets are discriminated by payment method type, exactly as rollout keys are.
        assert_eq!(
            build_scope(
                "merchant_1",
                "cybersource",
                "Authorize",
                common_enums::PaymentMethod::Wallet,
                Some(common_enums::PaymentMethodType::GooglePay)
            ),
            "merchant_1_cybersource_wallet_google_pay_Authorize"
        );
        // Refund keys carry no payment method.
        assert_eq!(
            build_scope(
                "merchant_1",
                "cybersource",
                "Execute",
                common_enums::PaymentMethod::Card,
                None
            ),
            "merchant_1_cybersource_Execute"
        );
    }

    #[test]
    fn scope_separates_independently_enabled_keys() {
        // Card and wallet are enabled by separate rollout keys, so a wallet cutover must not
        // take card traffic with it.
        let card = build_scope(
            "merchant_1",
            "cybersource",
            "Authorize",
            common_enums::PaymentMethod::Card,
            None,
        );
        let wallet = build_scope(
            "merchant_1",
            "cybersource",
            "Authorize",
            common_enums::PaymentMethod::Wallet,
            Some(common_enums::PaymentMethodType::GooglePay),
        );

        assert_ne!(card, wallet);
        assert_ne!(
            card,
            build_scope(
                "merchant_2",
                "cybersource",
                "Authorize",
                common_enums::PaymentMethod::Card,
                None
            )
        );
        assert_ne!(
            card,
            build_scope(
                "merchant_1",
                "adyen",
                "Authorize",
                common_enums::PaymentMethod::Card,
                None
            )
        );
        assert_ne!(
            card,
            build_scope(
                "merchant_1",
                "cybersource",
                "PSync",
                common_enums::PaymentMethod::Card,
                None
            )
        );
    }

    #[test]
    fn cutover_key_cannot_collide_with_a_rollout_config_key() {
        let key = cutover_key(&build_scope(
            "merchant_1",
            "cybersource",
            "Authorize",
            common_enums::PaymentMethod::Card,
            None,
        ));

        assert!(key.starts_with(consts::UCS_KILL_SWITCH_REDIS_PREFIX));
        assert!(!key.starts_with(consts::UCS_ROLLOUT_PERCENT_CONFIG_PREFIX));
    }

    #[test]
    fn failure_reasons_have_distinct_tags() {
        let tags = [
            UcsFailureReason::ResponseUndecodable.as_str(),
            UcsFailureReason::RequestUnbuildable.as_str(),
            UcsFailureReason::NotImplemented.as_str(),
            UcsFailureReason::FlowFailed.as_str(),
        ];
        let unique: std::collections::HashSet<_> = tags.iter().collect();

        assert_eq!(unique.len(), tags.len());
    }
}
