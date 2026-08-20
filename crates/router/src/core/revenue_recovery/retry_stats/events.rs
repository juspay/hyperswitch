use api_models::payments::PaymentRevenueRecoveryMetadata;
use common_enums::{CardNetwork, CardType, PaymentMethodType, StandardisedCode};
use hyperswitch_domain_models::{
    payments::payment_attempt::PaymentAttempt,
    revenue_recovery::retry_stats_cluster_key::{Dim, RetryStatsClusterKey},
};
use router_env::logger;
use time::PrimitiveDateTime;

use super::document::{EventSlots, StatsDelta};
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
        // the card funding type (credit/debit), taken from the attempt's payment method
        // subtype.
        let card_network = card_details.and_then(|card| card.card_network.clone());

        let dim_source = prev_attempt;
        // The middle dimension is the card funding type (credit/debit). Only the two
        // card funding subtypes map onto a `CardType`; anything else is `Unknown`.
        let card_type_dim = match dim_source.payment_method_subtype {
            Some(PaymentMethodType::Credit) => Dim::Val(CardType::Credit),
            Some(PaymentMethodType::Debit) => Dim::Val(CardType::Debit),
            _ => Dim::Unknown,
        };

        let error_code_dim =
            resolve_error_code_dim_from_attempt(state, dim_source, card_network).await;

        // The card ISIN is stored inside the attempt's `payment_method_data` for card
        // payments. Read it the same way `PaymentAttempt::extract_card_network` reads the
        // network, then resolve the issuer from it via the `cards_info` lookup.
        let card_isin = dim_source
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
    let Some(isin) = card_isin.map(str::trim).filter(|v| !v.is_empty()) else {
        return Dim::Unknown;
    };

    match state.store.get_card_info(isin).await {
        Ok(Some(card_info)) => Dim::from_event_value(card_info.card_issuer.as_deref()),
        Ok(None) => Dim::Unknown,
        Err(error) => {
            logger::warn!(
                ?error,
                "revenue_recovery_retry_stats: issuer lookup by isin failed"
            );
            Dim::Unknown
        }
    }
}

fn standardised_code_dim(code: Option<StandardisedCode>) -> Option<Dim<StandardisedCode>> {
    code.map(Dim::Val)
}

/// Resolve the error-code dimension for Scenario 2a / 2b.
///
/// v2 does not persist `standardised_code` onto the attempt (GSM resolution runs
/// only on v1 payment paths), so a persisted value is preferred but, when absent,
/// the standardised code is resolved live from the GSM table using the attempt's
/// connector + error code — mirroring how Scenario 1 resolves it on the webhook.
/// The raw connector error code is the final fallback.
async fn resolve_error_code_dim_from_attempt(
    state: &SessionState,
    payment_attempt: &PaymentAttempt,
    card_network: Option<CardNetwork>,
) -> Dim<StandardisedCode> {
    let Some(error) = payment_attempt.error.as_ref() else {
        return Dim::Unknown;
    };

    // 1. Prefer a persisted standardised code (future-proof once v2 persists it).
    if let Some(dim) = standardised_code_dim(error.standardised_code) {
        return dim;
    }

    // 2. Fall back to a live GSM lookup keyed on the attempt's connector + error code.
    //    Use the same flow/sub-flow (Payment/Authorize) the attempt path persists under,
    //    so a live-resolved code matches what a persisted attempt would carry.
    if let Some(connector) = payment_attempt.connector.clone() {
        let gsm_record = helpers::get_gsm_record(
            state,
            connector,
            consts::PAYMENT_FLOW_STR,
            consts::AUTHORIZE_FLOW_STR,
            Some(error.code.clone()),
            Some(error.message.clone()),
            None, // issuer_error_code
            card_network,
        )
        .await;
        if let Some(dim) =
            standardised_code_dim(gsm_record.and_then(|record| record.standardised_code))
        {
            return dim;
        }
    }

    // 3. No standardised code could be resolved. The raw connector error code is not a
    //    `StandardisedCode`, so the strictly-typed error-code dimension is `Unknown`.
    Dim::Unknown
}
