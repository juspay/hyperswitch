use common_utils::{
    ext_traits::StringExt,
    types::{AmountConvertor, MinorUnit, StringMinorUnitForConnector},
};
use diesel_models::enums as storage_enums;
use masking::Secret;
use time::OffsetDateTime;

use crate::types::storage::dispute::Dispute;

#[serde_with::skip_serializing_none]
#[derive(serde::Serialize, Debug)]
pub struct KafkaDisputeEvent<'a> {
    pub dispute_id: &'a String,
    pub dispute_amount: MinorUnit,
    pub currency: storage_enums::Currency,
    pub dispute_stage: &'a storage_enums::DisputeStage,
    pub dispute_status: &'a storage_enums::DisputeStatus,
    pub payment_id: &'a common_utils::id_type::PaymentId,
    pub attempt_id: &'a String,
    pub merchant_id: &'a common_utils::id_type::MerchantId,
    pub connector_status: &'a String,
    pub connector_dispute_id: &'a String,
    pub connector_reason: Option<&'a String>,
    pub connector_reason_code: Option<&'a String>,
    #[serde(default, with = "time::serde::timestamp::nanoseconds::option")]
    pub challenge_required_by: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::timestamp::nanoseconds::option")]
    pub connector_created_at: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::timestamp::nanoseconds::option")]
    pub connector_updated_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::timestamp::nanoseconds")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::timestamp::nanoseconds")]
    pub modified_at: OffsetDateTime,
    pub connector: &'a String,
    pub evidence: &'a Secret<serde_json::Value>,
    pub profile_id: Option<&'a common_utils::id_type::ProfileId>,
    pub merchant_connector_id: Option<&'a common_utils::id_type::MerchantConnectorAccountId>,
    pub organization_id: &'a common_utils::id_type::OrganizationId,
}

impl<'a> KafkaDisputeEvent<'a> {
    pub fn from_storage(dispute: &'a Dispute) -> Self {
        let currency = dispute.dispute_currency.unwrap_or(
            dispute
                .currency
                .to_uppercase()
                .parse_enum("Currency")
                .unwrap_or_default(),
        );
        Self {
            dispute_id: &dispute.dispute_id,
            dispute_amount: StringMinorUnitForConnector::convert_back(
                &StringMinorUnitForConnector,
                dispute.amount.clone(),
                currency,
            )
            .unwrap_or_else(|e| {
                router_env::logger::error!("Failed to convert dispute amount: {e:?}");
                MinorUnit::new(0)
            }),
            currency,
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
            organization_id: &dispute.organization_id,
        }
    }
}

impl super::KafkaMessage for KafkaDisputeEvent<'_> {
    fn key(&self) -> String {
        format!(
            "{}_{}_{}",
            self.merchant_id.get_string_repr(),
            self.payment_id.get_string_repr(),
            self.dispute_id
        )
    }

    fn event_type(&self) -> crate::events::EventType {
        crate::events::EventType::Dispute
    }
}
