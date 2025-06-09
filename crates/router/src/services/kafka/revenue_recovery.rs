use common_utils::{id_type, types::MinorUnit};
use time::PrimitiveDateTime;

#[derive(serde::Serialize, Debug)]
pub struct RevenueRecovery<'a> {
    pub merchant_id: &'a id_type::MerchantId,
    pub invoice_id: Option<String>,
    pub invoice_amount: MinorUnit,
    pub invoice_currency: &'a common_enums::Currency,
    pub invoice_due_date: Option<PrimitiveDateTime>,
    pub invoice_date: PrimitiveDateTime,
    pub invoice_address: Option<api_models::payments::Address>,
    pub attempt_id: String,
    pub attempt_amount: MinorUnit,
    pub attempt_currency: &'a common_enums::Currency,
    pub attempt_status: &'a common_enums::AttemptStatus,
    pub pg_error_code: Option<String>,
    pub network_advice_code: Option<String>,
    pub network_error_code: Option<String>,
    pub first_pg_error_code: Option<String>,
    pub first_network_advice_code: Option<String>,
    pub first_network_error_code: Option<String>,
    pub attempt_created_at: PrimitiveDateTime,
    pub payment_method_type: Option<&'a common_enums::PaymentMethod>,
    pub payment_method_subtype: Option<&'a common_enums::PaymentMethodType>,
    pub card_network: Option<&'a common_enums::CardNetwork>,
    pub card_issuer: Option<String>,
    pub retry_count: Option<u16>,
    pub payment_gateway: Option<common_enums::connector_enums::Connector>,
}

impl super::KafkaMessage for RevenueRecovery<'_> {
    fn key(&self) -> String {
                    self.attempt_id.to_string()
    }

    fn event_type(&self) -> crate::events::EventType {
        crate::events::EventType::RevenueRecovery
    }
}
