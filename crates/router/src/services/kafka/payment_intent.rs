use data_models::payments::PaymentIntent;
use diesel_models::enums as storage_enums;
use time::OffsetDateTime;

#[derive(serde::Serialize, Debug)]
pub struct KafkaPaymentIntent<'a> {
    pub payment_id: &'a String,
    pub merchant_id: &'a String,
    pub status: storage_enums::IntentStatus,
    pub amount: i64,
    pub currency: Option<storage_enums::Currency>,
    pub amount_captured: Option<i64>,
    pub customer_id: Option<&'a String>,
    pub description: Option<&'a String>,
    pub return_url: Option<&'a String>,
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
        }
    }
}

impl<'a> super::KafkaMessage for KafkaPaymentIntent<'a> {
    fn key(&self) -> String {
        format!("{}_{}", self.merchant_id, self.payment_id)
    }

    fn creation_timestamp(&self) -> Option<i64> {
        Some(self.modified_at.unix_timestamp())
    }

    fn event_type(&self) -> crate::events::EventType {
        crate::events::EventType::PaymentIntent
    }
}
