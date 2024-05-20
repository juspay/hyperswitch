use diesel_models::{enums as storage_enums, refund::Refund};
use time::OffsetDateTime;

#[derive(serde::Serialize, Debug)]
pub struct KafkaRefund<'a> {
    pub internal_reference_id: &'a String,
    pub refund_id: &'a String, //merchant_reference id
    pub payment_id: &'a String,
    pub merchant_id: &'a String,
    pub connector_transaction_id: &'a String,
    pub connector: &'a String,
    pub connector_refund_id: Option<&'a String>,
    pub external_reference_id: Option<&'a String>,
    pub refund_type: &'a storage_enums::RefundType,
    pub total_amount: &'a i64,
    pub currency: &'a storage_enums::Currency,
    pub refund_amount: &'a i64,
    pub refund_status: &'a storage_enums::RefundStatus,
    pub sent_to_gateway: &'a bool,
    pub refund_error_message: Option<&'a String>,
    pub refund_arn: Option<&'a String>,
    #[serde(default, with = "time::serde::timestamp")]
    pub created_at: OffsetDateTime,
    #[serde(default, with = "time::serde::timestamp")]
    pub modified_at: OffsetDateTime,
    pub description: Option<&'a String>,
    pub attempt_id: &'a String,
    pub refund_reason: Option<&'a String>,
    pub refund_error_code: Option<&'a String>,
}

impl<'a> KafkaRefund<'a> {
    pub fn from_storage(refund: &'a Refund) -> Self {
        Self {
            internal_reference_id: &refund.internal_reference_id,
            refund_id: &refund.refund_id,
            payment_id: &refund.payment_id,
            merchant_id: &refund.merchant_id,
            connector_transaction_id: &refund.connector_transaction_id,
            connector: &refund.connector,
            connector_refund_id: refund.connector_refund_id.as_ref(),
            external_reference_id: refund.external_reference_id.as_ref(),
            refund_type: &refund.refund_type,
            total_amount: &refund.total_amount,
            currency: &refund.currency,
            refund_amount: &refund.refund_amount,
            refund_status: &refund.refund_status,
            sent_to_gateway: &refund.sent_to_gateway,
            refund_error_message: refund.refund_error_message.as_ref(),
            refund_arn: refund.refund_arn.as_ref(),
            created_at: refund.created_at.assume_utc(),
            modified_at: refund.updated_at.assume_utc(),
            description: refund.description.as_ref(),
            attempt_id: &refund.attempt_id,
            refund_reason: refund.refund_reason.as_ref(),
            refund_error_code: refund.refund_error_code.as_ref(),
        }
    }
}

impl<'a> super::KafkaMessage for KafkaRefund<'a> {
    fn key(&self) -> String {
        format!(
            "{}_{}_{}_{}",
            self.merchant_id, self.payment_id, self.attempt_id, self.refund_id
        )
    }

    fn event_type(&self) -> crate::events::EventType {
        crate::events::EventType::Refund
    }
}
