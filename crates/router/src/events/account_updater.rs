use common_enums::CardNetwork;
use common_utils::id_type;
use hyperswitch_domain_models::payment_method_data::PaymentMethodsData;
use serde::Serialize;
use time::OffsetDateTime;

use super::EventType;
use crate::{
    core::account_updater::types::{
        AccountUpdaterFailure, AccountUpdaterTerminalState, RefreshOutcome, SkipReason,
    },
    services::kafka::KafkaMessage,
    types::domain,
};

const EVENT_SCHEMA_VERSION: &str = "1.0.0";

/// Did the evaluation run at all. Orthogonal to what the card network said, so that
/// "we could not ask" and "the issuer reported no change" stay separable.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountUpdaterEvaluationStatus {
    Skipped,
    Failed,
    Completed,
}

/// One row per `force_sync=true` request, including evaluations that never reached the provider.
///
/// Carries no card-derived value. `SyncCard` has no `Debug`, and `KafkaMessage` requires it, so a
/// field of that type here would not compile.
#[derive(Debug, Serialize)]
pub struct KafkaAccountUpdaterEvent<'a> {
    pub event_schema_version: &'static str,
    pub request_id: Option<String>,
    pub merchant_id: &'a id_type::MerchantId,
    pub profile_id: &'a id_type::ProfileId,
    pub payment_method_id: &'a id_type::GlobalPaymentMethodId,
    pub provider: &'static str,
    pub card_network: Option<CardNetwork>,
    pub evaluation_status: AccountUpdaterEvaluationStatus,
    pub skip_category: Option<SkipReason>,
    pub failure_category: Option<AccountUpdaterFailure>,
    pub updater_outcome: Option<RefreshOutcome>,
    pub latency_ms: u128,
    pub created_at: i128,
}

impl<'a> KafkaAccountUpdaterEvent<'a> {
    pub fn new(
        request_id: Option<String>,
        merchant_id: &'a id_type::MerchantId,
        profile_id: &'a id_type::ProfileId,
        payment_method: &'a domain::PaymentMethod,
        provider: &'static str,
        terminal_state: AccountUpdaterTerminalState,
        latency_ms: u128,
    ) -> Self {
        let (evaluation_status, skip_category, failure_category, updater_outcome) =
            match terminal_state {
                AccountUpdaterTerminalState::Skipped(reason) => (
                    AccountUpdaterEvaluationStatus::Skipped,
                    Some(reason),
                    None,
                    None,
                ),
                AccountUpdaterTerminalState::Failed(failure) => (
                    AccountUpdaterEvaluationStatus::Failed,
                    None,
                    Some(failure),
                    None,
                ),
                AccountUpdaterTerminalState::Refreshed(outcome) => (
                    AccountUpdaterEvaluationStatus::Completed,
                    None,
                    None,
                    Some(outcome),
                ),
            };

        Self {
            event_schema_version: EVENT_SCHEMA_VERSION,
            request_id,
            merchant_id,
            profile_id,
            payment_method_id: payment_method.get_id(),
            provider,
            card_network: stored_card_network(payment_method),
            evaluation_status,
            skip_category,
            failure_category,
            updater_outcome,
            latency_ms,
            created_at: OffsetDateTime::now_utc().unix_timestamp_nanos(),
        }
    }
}

/// Read from stored metadata rather than the terminal state, which holds no card-derived value.
fn stored_card_network(payment_method: &domain::PaymentMethod) -> Option<CardNetwork> {
    match payment_method
        .payment_method_data
        .clone()
        .map(|payment_method_data| payment_method_data.into_inner())
    {
        Some(PaymentMethodsData::Card(card_details)) => card_details.card_network,
        _ => None,
    }
}

impl KafkaMessage for KafkaAccountUpdaterEvent<'_> {
    /// Keyed on the payment method so a card's rows stay ordered within a partition.
    fn key(&self) -> String {
        self.payment_method_id.get_string_repr().to_owned()
    }

    fn event_type(&self) -> EventType {
        EventType::AccountUpdater
    }
}
