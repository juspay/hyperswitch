use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::captures};

#[derive(Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Serialize, Deserialize)]
#[diesel(table_name = captures)]
#[diesel(primary_key(capture_id))]
pub struct Capture {
    pub capture_id: String,
    pub payment_id: String,
    pub merchant_id: String,
    pub status: storage_enums::CaptureStatus,
    pub amount: i64,
    pub currency: Option<storage_enums::Currency>,
    pub connector: Option<String>,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub error_reason: Option<String>,
    pub tax_amount: Option<i64>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    pub authorized_attempt_id: String,
    pub capture_sequence: i16,
    pub connector_transaction_id: Option<String>,
}

#[derive(
    Clone, Debug, Default, Insertable, router_derive::DebugAsDisplay, Serialize, Deserialize,
)]
#[diesel(table_name = captures)]
pub struct CaptureNew {
    pub capture_id: String,
    pub payment_id: String,
    pub merchant_id: String,
    pub status: storage_enums::CaptureStatus,
    pub amount: i64,
    pub currency: Option<storage_enums::Currency>,
    pub connector: Option<String>,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub error_reason: Option<String>,
    pub tax_amount: Option<i64>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created_at: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub modified_at: Option<PrimitiveDateTime>,
    pub authorized_attempt_id: String,
    pub capture_sequence: i16,
    pub connector_transaction_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CaptureUpdate {
    StatusUpdate {
        status: storage_enums::CaptureStatus,
    },
    ResponseUpdate {
        status: Option<storage_enums::CaptureStatus>,
        connector_transaction_id: Option<String>,
        error_code: Option<String>,
        error_message: Option<String>,
        error_reason: Option<String>,
    },
    ErrorUpdate {
        status: Option<storage_enums::CaptureStatus>,
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
    pub capture_sequence: Option<i16>,
    pub connector_transaction_id: Option<String>,
}

impl CaptureUpdate {
    pub fn apply_changeset(self, source: Capture) -> Capture {
        let capture_update: CaptureUpdateInternal = self.into();
        Capture {
            status: capture_update.status.unwrap_or(source.status),
            error_message: capture_update.error_message.or(source.error_message),
            error_code: capture_update.error_code.or(source.error_code),
            error_reason: capture_update.error_reason.or(source.error_reason),
            modified_at: common_utils::date_time::now(),
            capture_sequence: capture_update
                .capture_sequence
                .unwrap_or(source.capture_sequence),
            ..source
        }
    }
}

impl From<CaptureUpdate> for CaptureUpdateInternal {
    fn from(payment_attempt_child_update: CaptureUpdate) -> Self {
        let now = Some(common_utils::date_time::now());
        match payment_attempt_child_update {
            CaptureUpdate::StatusUpdate { status } => Self {
                status: Some(status),
                modified_at: now,
                ..Self::default()
            },
            CaptureUpdate::ResponseUpdate {
                status,
                connector_transaction_id,
                error_code,
                error_message,
                error_reason,
            } => Self {
                status,
                connector_transaction_id,
                error_code,
                error_message,
                error_reason,
                modified_at: now,
                ..Self::default()
            },
            CaptureUpdate::ErrorUpdate {
                status,
                error_code,
                error_message,
                error_reason,
            } => Self {
                status,
                error_code,
                error_message,
                error_reason,
                modified_at: now,
                ..Self::default()
            },
        }
    }
}
