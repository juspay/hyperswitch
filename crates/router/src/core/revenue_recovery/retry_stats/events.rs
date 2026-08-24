use api_models::payments::PaymentRevenueRecoveryMetadata;
use common_enums::{CardNetwork, CardType, StandardisedCode};
use hyperswitch_domain_models::{
    payments::payment_attempt::PaymentAttempt,
    revenue_recovery::{
        retry_stats_cluster_key::{Dim, RetryStatsClusterKey},
        retry_stats_document::{EventSlots, StatsDelta},
    },
};
use router_env::logger;
use time::PrimitiveDateTime;

use crate::{consts, core::payments::helpers, routes::SessionState};

pub struct RetryOutcomeEvent {
    pub key: RetryStatsClusterKey,
    pub delta: StatsDelta,
    pub success: bool,
}

impl RetryOutcomeEvent {
    /// Method-agnostic core: build an event from already-resolved dimensions.
    fn build(
        error_code_dim: Dim<StandardisedCode>,
        card_type_dim: Dim<CardType>,
        issuer_dim: Dim<String>,
        success: bool,
        created_at: PrimitiveDateTime,
    ) -> Self {
        let slots = EventSlots::from_utc(created_at);
        let delta = StatsDelta::for_event(slots, success);

        Self {
            key: RetryStatsClusterKey::leaf(error_code_dim, card_type_dim, issuer_dim),
            delta,
            success,
        }
    }

    /// The retry's fate (`success`) and event time
    /// come from the freshly-resolved `payment_attempt`, while the cluster dims
    /// (error_code, card_type, issuer) are read from `prev_attempt`, the failed
    /// attempt that triggered this retry. A retry outcome is only meaningful
    /// relative to a prior attempt, so `prev_attempt` is required — callers that
    /// have no previous attempt do not record.
    pub async fn from_attempt(
        state: &SessionState,
        payment_attempt: &PaymentAttempt,
        prev_attempt: &PaymentAttempt,
        revenue_recovery_metadata: &PaymentRevenueRecoveryMetadata,
        success: bool,
    ) -> Self {
        let card_details = revenue_recovery_metadata
            .billing_connector_payment_method_details
            .as_ref()
            .and_then(|details| details.get_billing_connector_card_info());

        let card_network = card_details.and_then(|card| card.card_network.clone());

        let error_code_dim =
            resolve_error_code_dim_from_attempt(state, prev_attempt, card_network).await;

        Self::build(
            error_code_dim,
            Dim::Unknown,
            Dim::Unknown,
            success,
            payment_attempt.created_at,
        )
    }
}

/// Resolve the error-code dimension.
///
/// The standardised code is resolved from the GSM table using the attempt's
/// connector + error code, keyed on the Payment/Authorize flow. When no GSM record matches,
/// the strictly-typed dimension is `Unknown`.
async fn resolve_error_code_dim_from_attempt(
    state: &SessionState,
    payment_attempt: &PaymentAttempt,
    card_network: Option<CardNetwork>,
) -> Dim<StandardisedCode> {
    match (
        payment_attempt.error.as_ref(),
        payment_attempt.connector.clone(),
    ) {
        (Some(error), Some(connector)) => {
            let gsm_record = helpers::get_gsm_record(
                state,
                connector,
                consts::PAYMENT_FLOW_STR,
                consts::AUTHORIZE_FLOW_STR,
                Some(error.code.clone()),
                Some(error.message.clone()),
                error.network_decline_code.clone(),
                card_network,
            )
            .await;

            match gsm_record.and_then(|record| record.standardised_code) {
                Some(code) => Dim::Val(code),
                None => {
                    logger::warn!(
                        connector_error_code = error.code,
                        "revenue_recovery_retry_stats: no standardised code resolved from GSM for \
                         the attempt's error; recording error_code dimension as Unknown"
                    );
                    Dim::Unknown
                }
            }
        }
        _ => {
            logger::warn!(
                has_error = payment_attempt.error.is_some(),
                has_connector = payment_attempt.connector.is_some(),
                "revenue_recovery_retry_stats: attempt is missing error and/or connector; \
                 recording error_code dimension as Unknown"
            );
            Dim::Unknown
        }
    }
}
