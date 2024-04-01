use common_utils::pii;
use data_models::payouts::{payout_attempt::PayoutAttempt, payouts::Payouts};
use diesel_models::enums as storage_enums;
use time::OffsetDateTime;

#[derive(serde::Serialize, Debug)]
pub struct KafkaPayouts<'a> {
    pub payout_id: &'a String,
    pub payout_attempt_id: &'a String,
    pub merchant_id: &'a String,
    pub customer_id: &'a String,
    pub address_id: &'a String,
    pub profile_id: &'a String,
    pub payout_type: storage_enums::PayoutType,
    pub amount: i64,
    pub destination_currency: storage_enums::Currency,
    pub source_currency: storage_enums::Currency,
    pub description: Option<&'a String>,
    pub recurring: bool,
    pub auto_fulfill: bool,
    pub return_url: Option<&'a String>,
    pub entity_type: storage_enums::PayoutEntityType,
    pub metadata: Option<pii::SecretSerdeValue>,
    #[serde(with = "time::serde::timestamp")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::timestamp")]
    pub last_modified_at: OffsetDateTime,
    pub attempt_count: i16,
    pub status: storage_enums::PayoutStatus,

    pub connector: Option<&'a String>,
    pub connector_payout_id: &'a String,
    pub is_eligible: Option<bool>,
    pub error_message: Option<&'a String>,
    pub error_code: Option<&'a String>,
    pub business_country: Option<storage_enums::CountryAlpha2>,
    pub business_label: Option<&'a String>,
    pub merchant_connector_id: Option<&'a String>,
}

impl<'a> KafkaPayouts<'a> {
    pub fn from_storage(payouts: &'a Payouts, payout_attempt: &'a PayoutAttempt) -> Self {
        Self {
            payout_id: &payouts.payout_id,
            payout_attempt_id: &payout_attempt.payout_attempt_id,
            merchant_id: &payouts.merchant_id,
            customer_id: &payouts.customer_id,
            address_id: &payouts.address_id,
            profile_id: &payouts.profile_id,
            payout_type: payouts.payout_type,
            amount: payouts.amount,
            destination_currency: payouts.destination_currency,
            source_currency: payouts.source_currency,
            description: payouts.description.as_ref(),
            recurring: payouts.recurring,
            auto_fulfill: payouts.auto_fulfill,
            return_url: payouts.return_url.as_ref(),
            entity_type: payouts.entity_type,
            metadata: payouts.metadata.clone(),
            created_at: payouts.created_at.assume_utc(),
            last_modified_at: payouts.last_modified_at.assume_utc(),
            attempt_count: payouts.attempt_count,
            status: payouts.status,
            connector: payout_attempt.connector.as_ref(),
            connector_payout_id: &payout_attempt.connector_payout_id,
            is_eligible: payout_attempt.is_eligible,
            error_message: payout_attempt.error_message.as_ref(),
            error_code: payout_attempt.error_code.as_ref(),
            business_country: payout_attempt.business_country,
            business_label: payout_attempt.business_label.as_ref(),
            merchant_connector_id: payout_attempt.merchant_connector_id.as_ref(),
        }
    }
}

impl<'a> super::KafkaMessage for KafkaPayouts<'a> {
    fn key(&self) -> String {
        format!("{}_{}", self.merchant_id, self.payout_attempt_id)
    }

    fn creation_timestamp(&self) -> Option<i64> {
        Some(self.last_modified_at.unix_timestamp())
    }

    fn event_type(&self) -> crate::events::EventType {
        crate::events::EventType::Payouts
    }
}
