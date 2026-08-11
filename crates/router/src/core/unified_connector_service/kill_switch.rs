//! Automatic cutover from the Unified Connector Service back to the direct connector integration.
//!
//! Hyperswitch's direct connector integrations have served live merchants for years. UCS is being
//! rolled out behind `ucs_rollout_config` keys, and promoting one of those keys from shadow to
//! primary is the moment a UCS regression starts reaching a merchant. This module bounds that
//! exposure: when a rollout scope fails in a way that will keep failing, its traffic is cut back
//! to the direct integration without waiting for a human.
//!
//! The switch is deliberately asymmetric. Cutting over unnecessarily costs a merchant nothing —
//! they get the integration they had before UCS existed — while failing to cut over exposes live
//! traffic to a regression. Every ambiguous outcome resolves towards the direct path.
//!
//! # Why the cutover lives in redis
//!
//! A cutover is runtime state: an observation that something broke at 03:14. The
//! `ucs_rollout_config` rows are the opposite — a declaration of what the migration intends. This
//! module never writes to the `configs` table, so re-applying the intended config set (promoting
//! the next batch, re-running a runbook, restoring from a known-good list) cannot silently
//! overwrite a cutover, and a defect in this module cannot corrupt a row that decides routing.
//!
//! # Cost
//!
//! The cutover lookup only runs when the rollout config already resolved to **primary** —
//! shadow traffic never reaches it. So the added redis read is scoped to exactly the scopes being
//! protected, not to every request.

use common_enums::ExecutionMode;
use error_stack::ResultExt;
use hyperswitch_interfaces::unified_connector_service::transformers::UnifiedConnectorServiceError;
use router_env::logger;

use crate::{
    consts,
    core::{errors, metrics, payments::helpers::is_ucs_enabled},
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
/// The switch fires on the **first** qualifying failure — no counting, no threshold — which is
/// only sound because this function admits nothing whose first occurrence is uninformative:
///
/// - **Deterministic failures cut over.** A response Hyperswitch cannot decode, a request it
///   cannot build, a flow UCS does not implement. The second occurrence tells you nothing the
///   first did not; waiting for it only means more merchants hit the same wall. A threshold would
///   be actively harmful here — at any realistic threshold, a fully broken low-volume scope never
///   reaches it, so the long tail of the rollout would be silently unprotected.
///
/// - **Transient and availability failures do not.** `ConnectionError` and transport-level
///   `TonicStatus` mean UCS is unreachable or unhealthy — a fleet-wide fact that would trip every
///   scope at once during an ordinary rolling deploy, reverting the whole migration and telling
///   nobody which scope was actually broken. That is a global condition and needs a global remedy,
///   not one cutover per scope.
///
/// - **Connector outcomes never do.** `ConnectorError` is the connector answering: a decline, an
///   expired card, or a timeout (`decode_connector_timeout` builds a `ConnectorError` with status
///   504). All of it would fail identically on the direct path, so cutting over neither helps the
///   merchant nor indicates anything about UCS.
///
/// Matched exhaustively on purpose: a new `UnifiedConnectorServiceError` variant should fail to
/// compile here and force an explicit decision, rather than fall into a catch-all that silently
/// picks a side.
pub fn classify_failure(error: &UnifiedConnectorServiceError) -> Option<UcsFailureReason> {
    use UnifiedConnectorServiceError as E;

    match error {
        // Hyperswitch could not decode what UCS returned. Deterministic for this scope.
        E::ResponseDeserializationFailed | E::ParsingFailed => {
            Some(UcsFailureReason::ResponseUndecodable)
        }

        // Hyperswitch could not build a valid request, or could not resolve what to send it to.
        // The mapping between the Hyperswitch and UCS models is wrong for this scope.
        E::RequestEncodingFailed
        | E::RequestEncodingFailedWithReason(_)
        | E::MissingRequiredField { .. }
        | E::MissingRequiredFields { .. }
        | E::MissingConnectorName
        | E::InvalidConnectorName
        | E::InvalidDataFormat { .. }
        | E::FailedToObtainAuthType
        | E::HeaderInjectionFailed(_) => Some(UcsFailureReason::RequestUnbuildable),

        // UCS does not implement this flow, so the rollout key promoting it was wrong.
        E::NotImplemented(_) => Some(UcsFailureReason::NotImplemented),

        // Transport and availability. Fleet-wide conditions: a rolling UCS deploy produces these
        // across every scope at once, so cutting over on them would revert the entire migration
        // and identify nothing. `ucs_enabled` is the lever for a global UCS problem.
        E::ConnectionError(_) | E::TonicStatus { .. } => None,

        // The connector answered — a decline, an expired card, or a timeout, since
        // `decode_connector_timeout` builds a `ConnectorError` carrying status 504. All of it
        // would fail identically on the direct path.
        E::ConnectorError(_) => None,

        // Per-flow failure markers. None of these are constructed today outside the webhook
        // path, but they carry no detail beyond "this flow failed", so they are treated as
        // deterministic: over-cutting costs a merchant nothing, under-cutting does not.
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

/// Scope a single cutover covers.
///
/// Deliberately coarser than every `ucs_rollout_config` key shape, which additionally discriminate
/// on payment method and payment method type:
///
/// - **A cutover cannot be evaded by payment method.** If UCS is broken for a merchant's wallet
///   authorizations on a connector, that merchant's card authorizations on the same connector are
///   cut over too. Wider than strictly necessary, in the direction that costs nothing.
/// - **The recording site and the enforcement site compute it identically**, from the same helper
///   and the same inputs. A mismatch between them would silently disable the switch, so there is
///   exactly one way to build it.
pub fn build_scope(merchant_id: &str, connector_name: &str, flow_name: &str) -> String {
    format!("{merchant_id}_{connector_name}_{flow_name}")
}

/// Redis key holding the cutover for a scope.
fn cutover_key(scope: &str) -> String {
    format!("{}_{scope}", consts::UCS_KILL_SWITCH_REDIS_PREFIX)
}

/// Whether the kill switch is armed. Cached config lookup, same as `UCS_ENABLED`.
async fn is_armed(state: &SessionState) -> bool {
    is_ucs_enabled(state, consts::UCS_KILL_SWITCH_ENABLED).await
}

/// Whether UCS has been cut off for this scope.
///
/// Fails closed: a redis error resolves to "cut over", sending traffic to the direct integration.
/// That inverts the usual fail-open convention deliberately — the fallback here is the integration
/// Hyperswitch served before UCS existed, so an unnecessary fallback is not merchant-visible,
/// while a missed one is.
///
/// Only reached once the rollout config has already resolved to primary, so shadow traffic never
/// pays for this lookup.
pub async fn is_cut_over(state: &SessionState, scope: &str) -> bool {
    if !is_armed(state).await {
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
/// Never returns an error and never propagates one: this runs on a path that has already failed,
/// and a kill switch that can itself fail a payment is worse than the regression it guards
/// against.
pub async fn record_failure(
    state: &SessionState,
    merchant_id: &str,
    connector_name: &str,
    flow_name: &str,
    execution_mode: ExecutionMode,
    error: &UnifiedConnectorServiceError,
) {
    // Only the path serving merchant traffic can cut over. Shadow failures are the shadow
    // validation pipeline's concern, and counting them would arrive pre-tripped across every
    // scope currently mirroring.
    if !matches!(execution_mode, ExecutionMode::Primary) {
        return;
    }

    let Some(reason) = classify_failure(error) else {
        return;
    };

    let scope = build_scope(merchant_id, connector_name, flow_name);

    metrics::UCS_KILL_SWITCH_FAILURE.add(
        1,
        router_env::metric_attributes!(
            ("connector", connector_name.to_string()),
            ("flow", flow_name.to_string()),
            ("reason", reason.as_str())
        ),
    );

    // Disarmed: the metric above still reports what *would* have cut over, which is how the
    // classifier is validated against real traffic before the switch is armed.
    if !is_armed(state).await {
        logger::warn!(
            ucs_kill_switch_scope = %scope,
            reason = reason.as_str(),
            "ucs_kill_switch: qualifying failure observed but the switch is disarmed"
        );
        return;
    }

    cut_over(state, &scope, reason, error).await;
}

/// Writes the cutover for `scope`. `SET NX` makes this exactly-once fleet-wide: concurrent
/// failures all attempt it, one wins, the rest are a no-op.
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

    // The record is what an on-call engineer reads first, so it carries enough to pull the
    // originating request out of the logs rather than just asserting that something broke.
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

/// Clears the cutover for a scope, returning it to whatever its rollout config says.
///
/// Kept an explicit operator action rather than an automatic recovery: nothing should silently
/// put a scope that already burned live traffic back on UCS.
pub async fn reset_cut_over(
    state: SessionState,
    merchant_id: common_utils::id_type::MerchantId,
    connector_name: String,
    flow_name: String,
) -> errors::RouterResponse<()> {
    let scope = build_scope(merchant_id.get_string_repr(), &connector_name, &flow_name);

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
///
/// Without this the switch is unusable at the scale it guards — there are over a thousand
/// provisioned rollout keys, and an on-call engineer cannot reconstruct which ones are cut over
/// by grepping logs.
pub async fn list_cut_over_scopes(state: SessionState) -> errors::RouterResponse<Vec<String>> {
    let prefix = consts::UCS_KILL_SWITCH_REDIS_PREFIX;

    let keys = state
        .store
        .get_redis_conn()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get a redis connection to list UCS kill switch cutovers")?
        .scan(&format!("{prefix}_*").as_str().into(), Some(100), None)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to scan UCS kill switch cutovers")?;

    Ok(crate::services::ApplicationResponse::Json(keys))
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
        // would revert the whole migration and identify nothing.
        assert!(
            classify_failure(&UnifiedConnectorServiceError::ConnectionError(
                "dial".into()
            ))
            .is_none()
        );

        for code in [
            tonic::Code::Unavailable,
            tonic::Code::Internal,
            tonic::Code::DeadlineExceeded,
            tonic::Code::Unknown,
        ] {
            assert!(
                classify_failure(&UnifiedConnectorServiceError::TonicStatus {
                    code,
                    message: "upstream connect error".to_string(),
                })
                .is_none(),
                "tonic code {code:?} must not cut a scope over"
            );
        }
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
    fn scope_is_coarser_than_the_rollout_key() {
        // Rollout keys discriminate on payment method; the scope must not, or a cutover for one
        // payment method would leave the others on a connector already known to be broken.
        assert_eq!(
            build_scope("merchant_1", "cybersource", "Authorize"),
            "merchant_1_cybersource_Authorize"
        );
    }

    #[test]
    fn scope_separates_merchant_connector_and_flow() {
        let base = build_scope("merchant_1", "cybersource", "Authorize");

        assert_ne!(base, build_scope("merchant_2", "cybersource", "Authorize"));
        assert_ne!(base, build_scope("merchant_1", "adyen", "Authorize"));
        assert_ne!(base, build_scope("merchant_1", "cybersource", "PSync"));
    }

    #[test]
    fn cutover_key_cannot_collide_with_a_rollout_config_key() {
        let key = cutover_key(&build_scope("merchant_1", "cybersource", "Authorize"));

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
