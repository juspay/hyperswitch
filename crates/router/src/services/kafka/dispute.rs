use diesel_models::enums as storage_enums;
use masking::Secret;
use time::OffsetDateTime;

use crate::types::storage::dispute::Dispute;

#[derive(serde::Serialize, Debug)]
pub struct KafkaDispute<'a> {
    pub dispute_id: &'a String,
    pub amount: &'a String,
    pub currency: &'a String,
    pub dispute_stage: &'a storage_enums::DisputeStage,
    pub dispute_status: &'a storage_enums::DisputeStatus,
    pub payment_id: &'a String,
    pub attempt_id: &'a String,
    pub merchant_id: &'a String,
    pub connector_status: &'a String,
    pub connector_dispute_id: &'a String,
    pub connector_reason: Option<&'a String>,
    pub connector_reason_code: Option<&'a String>,
    #[serde(default, with = "time::serde::timestamp::option")]
    pub challenge_required_by: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::timestamp::option")]
    pub connector_created_at: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::timestamp::option")]
    pub connector_updated_at: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::timestamp")]
    pub created_at: OffsetDateTime,
    #[serde(default, with = "time::serde::timestamp")]
    pub modified_at: OffsetDateTime,
    pub connector: &'a String,
    pub evidence: &'a Secret<serde_json::Value>,
    pub profile_id: Option<&'a String>,
    pub merchant_connector_id: Option<&'a String>,
}

impl<'a> KafkaDispute<'a> {
    pub fn from_storage(dispute: &'a Dispute) -> Self {
        Self {
            dispute_id: &dispute.dispute_id,
            amount: &dispute.amount,
            currency: &dispute.currency,
            dispute_stage: &dispute.dispute_stage,
            dispute_status: &dispute.dispute_status,
            payment_id: &dispute.payment_id,
            attempt_id: &dispute.attempt_id,
            merchant_id: &dispute.merchant_id,
            connector_status: &dispute.connector_status,
            connector_dispute_id: &dispute.connector_dispute_id,
            connector_reason: dispute.connector_reason.as_ref(),
            connector_reason_code: dispute.connector_reason_code.as_ref(),
            challenge_required_by: dispute.challenge_required_by.map(|i| i.assume_utc()),
            connector_created_at: dispute.connector_created_at.map(|i| i.assume_utc()),
            connector_updated_at: dispute.connector_updated_at.map(|i| i.assume_utc()),
            created_at: dispute.created_at.assume_utc(),
            modified_at: dispute.modified_at.assume_utc(),
            connector: &dispute.connector,
            evidence: &dispute.evidence,
            profile_id: dispute.profile_id.as_ref(),
            merchant_connector_id: dispute.merchant_connector_id.as_ref(),
        }
    }
}

impl<'a> super::KafkaMessage for KafkaDispute<'a> {
    fn key(&self) -> String {
        format!(
            "{}_{}_{}",
            self.merchant_id, self.payment_id, self.dispute_id
        )
    }

    fn creation_timestamp(&self) -> Option<i64> {
        Some(self.modified_at.unix_timestamp())
    }
}
