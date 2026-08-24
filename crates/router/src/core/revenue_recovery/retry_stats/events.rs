use api_models::payments::PaymentRevenueRecoveryMetadata;
use common_enums::{CardNetwork, CardType, PaymentMethodType, StandardisedCode};
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

        // `card_network` is only needed for the live GSM lookup; the middle dimension is
        // the card funding type (credit/debit), taken from the revenue recovery
        // metadata's payment method subtype.
        let card_network = card_details.and_then(|card| card.card_network.clone());

        // The middle dimension is the card funding type (credit/debit), read from the
        // revenue recovery metadata's payment method subtype (the original billing
        // payment method). Only the two card funding subtypes map onto a `CardType`;
        // anything else is `Unknown`.
        let card_type_dim = match revenue_recovery_metadata.payment_method_subtype {
            PaymentMethodType::Credit => Dim::Val(CardType::Credit),
            PaymentMethodType::Debit => Dim::Val(CardType::Debit),
            _ => Dim::Unknown,
        };

        let error_code_dim =
            resolve_error_code_dim_from_attempt(state, prev_attempt, card_network).await;

        // The card ISIN is stored inside the attempt's `payment_method_data` for card
        // payments. Read it the same way `PaymentAttempt::extract_card_network` reads the
        // network, then resolve the issuer from it via the `cards_info` lookup.
        let card_isin = prev_attempt
            .get_payment_method_data()
            .ok()
            .flatten()
            .and_then(|data| data.get_additional_card_info())
            .and_then(|card| card.card_isin);
        let issuer_dim = resolve_issuer_dim(state, card_isin.as_deref()).await;

        Self::build(
            error_code_dim,
            card_type_dim,
            issuer_dim,
            success,
            payment_attempt.created_at,
        )
    }
}

/// Resolve the issuer dimension solely from the card ISIN via the `cards_info`
/// lookup table — the single source of truth for the issuer name. We deliberately
/// do not fall back to any webhook-provided issuer. A missing ISIN, no matching
/// `cards_info` row, or a lookup error all yield `Unknown`.
async fn resolve_issuer_dim(state: &SessionState, card_isin: Option<&str>) -> Dim<String> {
    match card_isin.map(str::trim).filter(|v| !v.is_empty()) {
        None => Dim::Unknown,
        Some(isin) => match state.store.get_card_info(isin).await {
            Ok(Some(card_info)) => Dim::from_event_value(card_info.card_issuer.as_deref()),
            Ok(None) => Dim::Unknown,
            Err(error) => {
                logger::warn!(
                    ?error,
                    "revenue_recovery_retry_stats: issuer lookup by isin failed"
                );
                Dim::Unknown
            }
        },
    }
}

fn standardised_code_dim(code: Option<StandardisedCode>) -> Option<Dim<StandardisedCode>> {
    code.map(Dim::Val)
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

            match standardised_code_dim(gsm_record.and_then(|record| record.standardised_code)) {
                Some(dim) => dim,
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
