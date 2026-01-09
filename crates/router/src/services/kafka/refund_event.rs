#[cfg(feature = "v2")]
use common_utils::pii;
#[cfg(feature = "v2")]
use common_utils::types::{self, ChargeRefunds};
use common_utils::{
    id_type,
    types::{ConnectorTransactionIdTrait, MinorUnit},
};
use diesel_models::{enums as storage_enums, refund::Refund};
use time::OffsetDateTime;

use crate::events;

#[cfg(feature = "v1")]
#[serde_with::skip_serializing_none]
#[derive(serde::Serialize, Debug)]
pub struct KafkaRefundEvent<'a> {
    pub internal_reference_id: &'a String,
    pub refund_id: &'a String, //merchant_reference id
    pub payment_id: &'a id_type::PaymentId,
    pub merchant_id: &'a id_type::MerchantId,
    pub connector_transaction_id: &'a String,
    pub connector: &'a String,
    pub connector_refund_id: Option<&'a String>,
    pub external_reference_id: Option<&'a String>,
    pub refund_type: &'a storage_enums::RefundType,
    pub total_amount: &'a MinorUnit,
    pub currency: &'a storage_enums::Currency,
    pub refund_amount: &'a MinorUnit,
    pub refund_status: &'a storage_enums::RefundStatus,
    pub sent_to_gateway: &'a bool,
    pub refund_error_message: Option<&'a String>,
    pub refund_arn: Option<&'a String>,
    #[serde(with = "time::serde::timestamp::nanoseconds")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::timestamp::nanoseconds")]
    pub modified_at: OffsetDateTime,
    pub description: Option<&'a String>,
    pub attempt_id: &'a String,
    pub refund_reason: Option<&'a String>,
    pub refund_error_code: Option<&'a String>,
    pub profile_id: Option<&'a id_type::ProfileId>,
    pub organization_id: &'a id_type::OrganizationId,
}

#[cfg(feature = "v1")]
impl<'a> KafkaRefundEvent<'a> {
    pub fn from_storage(refund: &'a Refund) -> Self {
        Self {
            internal_reference_id: &refund.internal_reference_id,
            refund_id: &refund.refund_id,
            payment_id: &refund.payment_id,
            merchant_id: &refund.merchant_id,
            connector_transaction_id: refund.get_connector_transaction_id(),
            connector: &refund.connector,
            connector_refund_id: refund.get_optional_connector_refund_id(),
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
            modified_at: refund.modified_at.assume_utc(),
            description: refund.description.as_ref(),
            attempt_id: &refund.attempt_id,
            refund_reason: refund.refund_reason.as_ref(),
            refund_error_code: refund.refund_error_code.as_ref(),
            profile_id: refund.profile_id.as_ref(),
            organization_id: &refund.organization_id,
        }
    }
}

#[cfg(feature = "v2")]
#[serde_with::skip_serializing_none]
#[derive(serde::Serialize, Debug)]
pub struct KafkaRefundEvent<'a> {
    pub refund_id: &'a id_type::GlobalRefundId,
    pub merchant_reference_id: &'a id_type::RefundReferenceId,
    pub payment_id: &'a id_type::GlobalPaymentId,
    pub merchant_id: &'a id_type::MerchantId,
    pub connector_transaction_id: &'a types::ConnectorTransactionId,
    pub connector: &'a String,
    pub connector_refund_id: Option<&'a types::ConnectorTransactionId>,
    pub external_reference_id: Option<&'a String>,
    pub refund_type: &'a storage_enums::RefundType,
    pub total_amount: &'a MinorUnit,
    pub currency: &'a storage_enums::Currency,
    pub refund_amount: &'a MinorUnit,
    pub refund_status: &'a storage_enums::RefundStatus,
    pub sent_to_gateway: &'a bool,
    pub refund_error_message: Option<&'a String>,
    pub refund_arn: Option<&'a String>,
    #[serde(with = "time::serde::timestamp::nanoseconds")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::timestamp::nanoseconds")]
    pub modified_at: OffsetDateTime,
    pub description: Option<&'a String>,
    pub attempt_id: &'a id_type::GlobalAttemptId,
    pub refund_reason: Option<&'a String>,
    pub refund_error_code: Option<&'a String>,
    pub profile_id: Option<&'a id_type::ProfileId>,
    pub organization_id: &'a id_type::OrganizationId,

    pub metadata: Option<&'a pii::SecretSerdeValue>,
    pub updated_by: &'a String,
    pub merchant_connector_id: Option<&'a id_type::MerchantConnectorAccountId>,
    pub charges: Option<&'a ChargeRefunds>,
    pub connector_refund_data: Option<&'a String>,
    pub connector_transaction_data: Option<&'a String>,
    pub split_refunds: Option<&'a common_types::refunds::SplitRefund>,
    pub unified_code: Option<&'a String>,
    pub unified_message: Option<&'a String>,
    pub processor_refund_data: Option<&'a String>,
    pub processor_transaction_data: Option<&'a String>,
}

#[cfg(feature = "v2")]
impl<'a> KafkaRefundEvent<'a> {
    pub fn from_storage(refund: &'a Refund) -> Self {
        let Refund {
            payment_id,
            merchant_id,
            connector_transaction_id,
            connector,
            connector_refund_id,
            external_reference_id,
            refund_type,
            total_amount,
            currency,
            refund_amount,
            refund_status,
            sent_to_gateway,
            refund_error_message,
            metadata,
            refund_arn,
            created_at,
            modified_at,
            description,
            attempt_id,
            refund_reason,
            refund_error_code,
            profile_id,
            updated_by,
            charges,
            organization_id,
            split_refunds,
            unified_code,
            unified_message,
            processor_refund_data,
            processor_transaction_data,
            id,
            merchant_reference_id,
            connector_id,
        } = refund;

        Self {
            refund_id: id,
            merchant_reference_id,
            payment_id,
            merchant_id,
            connector_transaction_id,
            connector,
            connector_refund_id: connector_refund_id.as_ref(),
            external_reference_id: external_reference_id.as_ref(),
            refund_type,
            total_amount,
            currency,
            refund_amount,
            refund_status,
            sent_to_gateway,
            refund_error_message: refund_error_message.as_ref(),
            refund_arn: refund_arn.as_ref(),
            created_at: created_at.assume_utc(),
            modified_at: modified_at.assume_utc(),
            description: description.as_ref(),
            attempt_id,
            refund_reason: refund_reason.as_ref(),
            refund_error_code: refund_error_code.as_ref(),
            profile_id: profile_id.as_ref(),
            organization_id,
            metadata: metadata.as_ref(),
            updated_by,
            merchant_connector_id: connector_id.as_ref(),
            charges: charges.as_ref(),
            connector_refund_data: processor_refund_data.as_ref(),
            connector_transaction_data: processor_transaction_data.as_ref(),
            split_refunds: split_refunds.as_ref(),
            unified_code: unified_code.as_ref(),
            unified_message: unified_message.as_ref(),
            processor_refund_data: processor_refund_data.as_ref(),
            processor_transaction_data: processor_transaction_data.as_ref(),
        }
    }
}

#[cfg(feature = "v1")]
impl super::KafkaMessage for KafkaRefundEvent<'_> {
    fn key(&self) -> String {
        format!(
            "{}_{}_{}_{}",
            self.merchant_id.get_string_repr(),
            self.payment_id.get_string_repr(),
            self.attempt_id,
            self.refund_id
        )
    }
    fn event_type(&self) -> events::EventType {
        events::EventType::Refund
    }
}

#[cfg(feature = "v2")]
impl super::KafkaMessage for KafkaRefundEvent<'_> {
    fn key(&self) -> String {
        format!(
            "{}_{}_{}_{}",
            self.merchant_id.get_string_repr(),
            self.payment_id.get_string_repr(),
            self.attempt_id.get_string_repr(),
            self.merchant_reference_id.get_string_repr()
        )
    }
    fn event_type(&self) -> events::EventType {
        events::EventType::Refund
    }
}
