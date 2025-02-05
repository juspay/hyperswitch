use common_utils::types::{ConnectorTransactionId, MinorUnit};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::captures};

#[derive(
    Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Selectable, Serialize, Deserialize,
)]
#[diesel(table_name = captures, primary_key(capture_id), check_for_backend(diesel::pg::Pg))]
pub struct Capture {
    pub capture_id: String,
    pub payment_id: common_utils::id_type::PaymentId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub status: storage_enums::CaptureStatus,
    pub amount: MinorUnit,
    pub currency: Option<storage_enums::Currency>,
    pub connector: String,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub error_reason: Option<String>,
    pub tax_amount: Option<MinorUnit>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    pub authorized_attempt_id: String,
    pub connector_capture_id: Option<ConnectorTransactionId>,
    pub capture_sequence: i16,
    // reference to the capture at connector side
    pub connector_response_reference_id: Option<String>,
    /// INFO: This field is deprecated and replaced by processor_capture_data
    pub connector_capture_data: Option<String>,
    pub processor_capture_data: Option<String>,
}

#[derive(Clone, Debug, Insertable, router_derive::DebugAsDisplay, Serialize, Deserialize)]
#[diesel(table_name = captures)]
pub struct CaptureNew {
    pub capture_id: String,
    pub payment_id: common_utils::id_type::PaymentId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub status: storage_enums::CaptureStatus,
    pub amount: MinorUnit,
    pub currency: Option<storage_enums::Currency>,
    pub connector: String,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub error_reason: Option<String>,
    pub tax_amount: Option<MinorUnit>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    pub authorized_attempt_id: String,
    pub connector_capture_id: Option<ConnectorTransactionId>,
    pub capture_sequence: i16,
    pub connector_response_reference_id: Option<String>,
    /// INFO: This field is deprecated and replaced by processor_capture_data
    pub connector_capture_data: Option<String>,
    pub processor_capture_data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CaptureUpdate {
    ResponseUpdate {
        status: storage_enums::CaptureStatus,
        connector_capture_id: Option<ConnectorTransactionId>,
        connector_response_reference_id: Option<String>,
        processor_capture_data: Option<String>,
    },
    ErrorUpdate {
        status: storage_enums::CaptureStatus,
        error_code: Option<String>,
        error_message: Option<String>,
        error_reason: Option<String>,
    },
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = captures)]
pub struct CaptureUpdateInternal {
    pub status: Option<storage_enums::CaptureStatus>,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub error_reason: Option<String>,
    pub modified_at: Option<PrimitiveDateTime>,
    pub connector_capture_id: Option<ConnectorTransactionId>,
    pub connector_response_reference_id: Option<String>,
    pub processor_capture_data: Option<String>,
}

impl CaptureUpdate {
    pub fn apply_changeset(self, source: Capture) -> Capture {
        let CaptureUpdateInternal {
            status,
            error_message,
            error_code,
            error_reason,
            modified_at: _,
            connector_capture_id,
            connector_response_reference_id,
            processor_capture_data,
        } = self.into();
        Capture {
            status: status.unwrap_or(source.status),
            error_message: error_message.or(source.error_message),
            error_code: error_code.or(source.error_code),
            error_reason: error_reason.or(source.error_reason),
            modified_at: common_utils::date_time::now(),
            connector_capture_id: connector_capture_id.or(source.connector_capture_id),
            connector_response_reference_id: connector_response_reference_id
                .or(source.connector_response_reference_id),
            processor_capture_data: processor_capture_data.or(source.processor_capture_data),
            ..source
        }
    }
}

impl From<CaptureUpdate> for CaptureUpdateInternal {
    fn from(payment_attempt_child_update: CaptureUpdate) -> Self {
        let now = Some(common_utils::date_time::now());
        match payment_attempt_child_update {
            CaptureUpdate::ResponseUpdate {
                status,
                connector_capture_id: connector_transaction_id,
                connector_response_reference_id,
                processor_capture_data,
            } => Self {
                status: Some(status),
                connector_capture_id: connector_transaction_id,
                modified_at: now,
                connector_response_reference_id,
                processor_capture_data,
                ..Self::default()
            },
            CaptureUpdate::ErrorUpdate {
                status,
                error_code,
                error_message,
                error_reason,
            } => Self {
                status: Some(status),
                error_code,
                error_message,
                error_reason,
                modified_at: now,
                ..Self::default()
            },
        }
    }
}
