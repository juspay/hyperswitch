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

    /// Build an event from the retry's freshly resolved attempt and the previous
    /// attempt's standardised error code, resolved once at ingestion time (see
    /// [`resolve_standardised_error_code`]) and carried on the workflow tracking data, so
    /// the recorder never re-reads the previous attempt from the payments store. A `None`
    /// code maps to the `Unknown` error-code dimension.
    pub fn from_attempt(
        payment_attempt: &PaymentAttempt,
        standardised_error_code: Option<StandardisedCode>,
        success: bool,
    ) -> Self {
        let error_code_dim = match standardised_error_code {
            Some(code) => Dim::Val(code),
            None => Dim::Unknown,
        };
        Self::build(
            error_code_dim,
            Dim::Unknown,
            Dim::Unknown,
            success,
            payment_attempt.created_at,
        )
    }
}

/// Resolve the standardised error code from an attempt
pub async fn resolve_standardised_error_code_from_attempt(
    state: &SessionState,
    payment_attempt: &PaymentAttempt,
    card_network: Option<CardNetwork>,
) -> Option<StandardisedCode> {
    resolve_standardised_error_code(
        state,
        payment_attempt.connector.clone(),
        payment_attempt
            .error
            .as_ref()
            .map(|error| error.code.clone()),
        payment_attempt
            .error
            .as_ref()
            .map(|error| error.message.clone()),
        payment_attempt
            .error
            .as_ref()
            .and_then(|error| error.network_decline_code.clone()),
        card_network,
    )
    .await
}

/// Resolve the standardised error code from an attempt's connector + error fields.
///
/// The code is resolved from the GSM table using the connector + error code, keyed on
/// the Payment/Authorize flow. Returns `None` when no GSM record matches or the
/// connector/error code is absent; the recorder maps that to the `Unknown` dimension.
///
/// Called at ingestion time (the revenue-recovery webhook) where these fields are in
/// hand, so the async retry-stats path never has to read the attempt back from the
/// payments store.
pub async fn resolve_standardised_error_code(
    state: &SessionState,
    connector: Option<String>,
    error_code: Option<String>,
    error_message: Option<String>,
    network_decline_code: Option<String>,
    card_network: Option<CardNetwork>,
) -> Option<StandardisedCode> {
    match (error_code, connector) {
        (Some(error_code), Some(connector)) => {
            let gsm_record = helpers::get_gsm_record(
                state,
                connector,
                consts::PAYMENT_FLOW_STR,
                consts::AUTHORIZE_FLOW_STR,
                Some(error_code.clone()),
                error_message,
                network_decline_code,
                card_network,
            )
            .await;

            let standardised_code = gsm_record.and_then(|record| record.standardised_code);
            if standardised_code.is_none() {
                logger::warn!(
                    connector_error_code = error_code,
                    "revenue_recovery_retry_stats: no standardised code resolved from GSM for \
                     the attempt's error; recording error_code dimension as Unknown"
                );
            }
            standardised_code
        }
        (error_code, connector) => {
            logger::warn!(
                has_error_code = error_code.is_some(),
                has_connector = connector.is_some(),
                "revenue_recovery_retry_stats: attempt is missing error code and/or connector; \
                 recording error_code dimension as Unknown"
            );
            None
        }
    }
}
