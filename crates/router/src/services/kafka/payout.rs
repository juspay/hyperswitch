use common_utils::pii;
use diesel_models::enums as storage_enums;
use time::OffsetDateTime;

#[cfg(feature = "payouts")]
use crate::types::storage::Payouts;

#[derive(serde::Serialize, Debug)]
pub struct KafkaPayout<'a> {
    pub payout_id: &'a String,
    pub merchant_id: &'a String,
    pub customer_id: &'a String,
    pub address_id: &'a String,
    pub payout_type: &'a storage_enums::PayoutType,
    pub payout_method_id: Option<&'a String>,
    pub amount: i64,
    pub destination_currency: &'a storage_enums::Currency,
    pub source_currency: &'a storage_enums::Currency,
    pub description: Option<&'a String>,
    pub recurring: bool,
    pub auto_fulfill: bool,
    pub return_url: Option<&'a String>,
    pub entity_type: &'a storage_enums::PayoutEntityType,
    pub metadata: Option<&'a pii::SecretSerdeValue>,
    #[serde(default, with = "time::serde::timestamp")]
    pub created_at: OffsetDateTime,
    #[serde(default, with = "time::serde::timestamp")]
    pub last_modified_at: OffsetDateTime,
    pub attempt_count: i16,
    pub profile_id: &'a String,
    pub status: &'a storage_enums::PayoutStatus,
}

#[cfg(feature = "payouts")]
impl<'a> KafkaPayout<'a> {
    pub fn from_storage(payout: &'a Payouts) -> Self {
        Self {
            payout_id: &payout.payout_id,
            merchant_id: &payout.merchant_id,
            customer_id: &payout.customer_id,
            address_id: &payout.address_id,
            payout_type: &payout.payout_type,
            payout_method_id: payout.payout_method_id.as_ref(),
            amount: payout.amount,
            destination_currency: &payout.destination_currency,
            source_currency: &payout.source_currency,
            description: payout.description.as_ref(),
            recurring: payout.recurring,
            auto_fulfill: payout.auto_fulfill,
            return_url: payout.return_url.as_ref(),
            entity_type: &payout.entity_type,
            metadata: payout.metadata.as_ref(),
            created_at: payout.created_at.assume_utc(),
            last_modified_at: payout.last_modified_at.assume_utc(),
            attempt_count: payout.attempt_count,
            profile_id: &payout.profile_id,
            status: &payout.status,
        }
    }
}

impl<'a> super::KafkaMessage for KafkaPayout<'a> {
    fn key(&self) -> String {
        format!("{}_{}", self.merchant_id, self.payout_id)
    }

    fn creation_timestamp(&self) -> Option<i64> {
        Some(self.last_modified_at.unix_timestamp())
    }

    fn event_type(&self) -> crate::events::EventType {
        crate::events::EventType::Payout
    }
}
