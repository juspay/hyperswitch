use common_utils::{id_type, types::MinorUnit};
use masking::Secret;
use time::OffsetDateTime;
#[derive(serde::Serialize, Debug)]
pub struct RevenueRecovery<'a> {
    pub merchant_id: &'a id_type::MerchantId,
    pub invoice_amount: MinorUnit,
    pub invoice_currency: &'a common_enums::Currency,
    #[serde(default, with = "time::serde::timestamp::nanoseconds::option")]
    pub invoice_due_date: Option<OffsetDateTime>,
    #[serde(with = "time::serde::timestamp::nanoseconds::option")]
    pub invoice_date: Option<OffsetDateTime>,
    pub billing_country: Option<&'a common_enums::CountryAlpha2>,
    pub billing_state: Option<Secret<String>>,
    pub billing_city: Option<Secret<String>>,
    pub attempt_amount: MinorUnit,
    pub attempt_currency: &'a common_enums::Currency,
    pub attempt_status: &'a common_enums::AttemptStatus,
    pub pg_error_code: Option<String>,
    pub network_advice_code: Option<String>,
    pub network_error_code: Option<String>,
    pub first_pg_error_code: Option<String>,
    pub first_network_advice_code: Option<String>,
    pub first_network_error_code: Option<String>,
    #[serde(default, with = "time::serde::timestamp::nanoseconds")]
    pub attempt_created_at: OffsetDateTime,
    pub payment_method_type: Option<&'a common_enums::PaymentMethod>,
    pub payment_method_subtype: Option<&'a common_enums::PaymentMethodType>,
    pub card_network: Option<&'a common_enums::CardNetwork>,
    pub card_issuer: Option<String>,
    pub retry_count: Option<i32>,
    pub payment_gateway: Option<common_enums::connector_enums::Connector>,
}

impl super::KafkaMessage for RevenueRecovery<'_> {
    fn key(&self) -> String {
        self.merchant_id.get_string_repr().to_string()
    }

    fn event_type(&self) -> crate::events::EventType {
        crate::events::EventType::RevenueRecovery
    }
}
