use common_utils::{crypto::Encryptable, hashing::HashedString, id_type, pii, types::MinorUnit};
use diesel_models::enums as storage_enums;
use hyperswitch_domain_models::payments::PaymentIntent;
use masking::{PeekInterface, Secret};
use serde_json::Value;
use time::OffsetDateTime;

#[derive(serde::Serialize, Debug)]
pub struct KafkaPaymentIntent<'a> {
    pub payment_id: &'a String,
    pub merchant_id: &'a id_type::MerchantId,
    pub status: storage_enums::IntentStatus,
    pub amount: MinorUnit,
    pub currency: Option<storage_enums::Currency>,
    pub amount_captured: Option<MinorUnit>,
    pub customer_id: Option<&'a id_type::CustomerId>,
    pub description: Option<&'a String>,
    pub return_url: Option<&'a String>,
    pub metadata: Option<String>,
    pub connector_id: Option<&'a String>,
    pub statement_descriptor_name: Option<&'a String>,
    pub statement_descriptor_suffix: Option<&'a String>,
    #[serde(with = "time::serde::timestamp")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::timestamp")]
    pub modified_at: OffsetDateTime,
    #[serde(default, with = "time::serde::timestamp::option")]
    pub last_synced: Option<OffsetDateTime>,
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub off_session: Option<bool>,
    pub client_secret: Option<&'a String>,
    pub active_attempt_id: String,
    pub business_country: Option<storage_enums::CountryAlpha2>,
    pub business_label: Option<&'a String>,
    pub attempt_count: i16,
    pub payment_confirm_source: Option<storage_enums::PaymentSource>,
    pub billing_details: Option<Encryptable<Secret<Value>>>,
    pub shipping_details: Option<Encryptable<Secret<Value>>>,
    pub customer_email: Option<HashedString<pii::EmailStrategy>>,
    pub feature_metadata: Option<&'a Value>,
    pub merchant_order_reference_id: Option<&'a String>,
}

impl<'a> KafkaPaymentIntent<'a> {
    pub fn from_storage(intent: &'a PaymentIntent) -> Self {
        Self {
            payment_id: &intent.payment_id,
            merchant_id: &intent.merchant_id,
            status: intent.status,
            amount: intent.amount,
            currency: intent.currency,
            amount_captured: intent.amount_captured,
            customer_id: intent.customer_id.as_ref(),
            description: intent.description.as_ref(),
            return_url: intent.return_url.as_ref(),
            metadata: intent.metadata.as_ref().map(|x| x.to_string()),
            connector_id: intent.connector_id.as_ref(),
            statement_descriptor_name: intent.statement_descriptor_name.as_ref(),
            statement_descriptor_suffix: intent.statement_descriptor_suffix.as_ref(),
            created_at: intent.created_at.assume_utc(),
            modified_at: intent.modified_at.assume_utc(),
            last_synced: intent.last_synced.map(|i| i.assume_utc()),
            setup_future_usage: intent.setup_future_usage,
            off_session: intent.off_session,
            client_secret: intent.client_secret.as_ref(),
            active_attempt_id: intent.active_attempt.get_id(),
            business_country: intent.business_country,
            business_label: intent.business_label.as_ref(),
            attempt_count: intent.attempt_count,
            payment_confirm_source: intent.payment_confirm_source,
            // TODO: use typed information here to avoid PII logging
            billing_details: None,
            shipping_details: None,
            customer_email: intent
                .customer_details
                .as_ref()
                .and_then(|value| value.get_inner().peek().as_object())
                .and_then(|obj| obj.get("email"))
                .and_then(|email| email.as_str())
                .map(|email| HashedString::from(Secret::new(email.to_string()))),
            feature_metadata: intent.feature_metadata.as_ref(),
            merchant_order_reference_id: intent.merchant_order_reference_id.as_ref(),
        }
    }
}

impl<'a> super::KafkaMessage for KafkaPaymentIntent<'a> {
    fn key(&self) -> String {
        format!("{}_{}", self.merchant_id.get_string_repr(), self.payment_id)
    }

    fn event_type(&self) -> crate::events::EventType {
        crate::events::EventType::PaymentIntent
    }
}
