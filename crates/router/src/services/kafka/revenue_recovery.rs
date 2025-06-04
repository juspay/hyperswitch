use common_utils::{id_type,types::MinorUnit};
use time::OffsetDateTime;
use common_utils::crypto::Encryptable;
use masking::PeekInterface;
use serde_json::Value;

#[derive(serde::Serialize, Debug)]
pub struct RevenueRecovery<'a>{
    pub merchant_id: &'a id_type::MerchantId,
    pub invoice_id: String,
    pub invoice_amount: MinorUnit,
    pub invoice_currency: &'a common_enums::Currency,
    #[serde(default, with = "time::serde::timestamp::option")]
    pub invoice_due_date: Option<OffsetDateTime>,
    #[serde(with = "time::serde::timestamp")]
    pub invoice_date: OffsetDateTime,
    pub invoice_address: Option<Encryptable<masking::Secret<Value>>>,
    pub attempt_id : String,
    pub attempt_amount: MinorUnit,
    pub attempt_currency: &'a common_enums::Currency,
    pub attempt_status: &'a common_enums::AttemptStatus,
    pub attempt_error_code: Option<String>,
    pub network_error_message: Option<String>,
    pub network_error_code: Option<String>,
    #[serde(with = "time::serde::timestamp")]
    pub attempt_created_at: OffsetDateTime,
    pub payment_method_type: Option<&'a common_enums::PaymentMethod>,
    pub payment_method_subtype: Option<&'a common_enums::PaymentMethodType>,
    pub card_network: Option<String>,
    pub card_issuer: Option<String>,
}

impl super::KafkaMessage for RevenueRecovery<'_> {
    fn key(&self) -> String {
        
            // self.merchant_id.get_string_repr(),
            // self.payment_id.get_string_repr(),
            self.attempt_id.to_string()
        
    }

    fn event_type(&self) -> crate::events::EventType {
        crate::events::EventType::RevenueRecovery
    }
}
