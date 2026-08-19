use common_enums::CardNetwork;
use common_utils::{errors::CustomResult, id_type};
use hyperswitch_domain_models::payment_method_data::PaymentMethodsData;
use serde::Serialize;
use time::OffsetDateTime;
use unified_connector_service_client::payments as payments_grpc;

use super::EventType;
use crate::{
    core::account_updater::types::AccountUpdaterError, services::kafka::KafkaMessage, types::domain,
};

#[derive(Debug, Serialize)]
pub struct KafkaAccountUpdaterEvent<'a> {
    pub request_id: Option<String>,
    pub merchant_id: &'a id_type::MerchantId,
    pub profile_id: &'a id_type::ProfileId,
    pub payment_method_id: &'a id_type::GlobalPaymentMethodId,
    pub card_network: Option<CardNetwork>,
    pub updater_outcome: Option<&'static str>,
    pub error_category: Option<&'a AccountUpdaterError>,
    pub latency_ms: u128,
    pub created_at: i128,
}

impl<'a> KafkaAccountUpdaterEvent<'a> {
    pub fn new(
        request_id: Option<String>,
        merchant_id: &'a id_type::MerchantId,
        profile_id: &'a id_type::ProfileId,
        payment_method: &'a domain::PaymentMethod,
        evaluation: &'a CustomResult<payments_grpc::CardRefreshOutcome, AccountUpdaterError>,
        latency_ms: u128,
    ) -> Self {
        let (updater_outcome, error_category) = match evaluation {
            Ok(outcome) => (Some(outcome.as_str_name()), None),
            Err(error) => (None, Some(error.current_context())),
        };

        Self {
            request_id,
            merchant_id,
            profile_id,
            payment_method_id: payment_method.get_id(),
            card_network: stored_card_network(payment_method),
            updater_outcome,
            error_category,
            latency_ms,
            created_at: OffsetDateTime::now_utc().unix_timestamp_nanos(),
        }
    }
}

fn stored_card_network(payment_method: &domain::PaymentMethod) -> Option<CardNetwork> {
    match payment_method
        .payment_method_data
        .as_ref()
        .map(|payment_method_data| payment_method_data.get_inner())
    {
        Some(PaymentMethodsData::Card(card_details)) => card_details.card_network.clone(),
        _ => None,
    }
}

impl KafkaMessage for KafkaAccountUpdaterEvent<'_> {
    fn key(&self) -> String {
        self.payment_method_id.get_string_repr().to_owned()
    }

    fn event_type(&self) -> EventType {
        EventType::AccountUpdater
    }
}
