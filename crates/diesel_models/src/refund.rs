use common_utils::pii;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::refund};

#[derive(
    Clone, Debug, Eq, Identifiable, Queryable, PartialEq, serde::Serialize, serde::Deserialize,
)]
#[diesel(table_name = refund)]
pub struct Refund {
    pub id: i32,
    pub internal_reference_id: String,
    pub refund_id: String, //merchant_reference id
    pub payment_id: String,
    pub merchant_id: String,
    pub connector_transaction_id: String,
    pub connector: String,
    pub connector_refund_id: Option<String>,
    pub external_reference_id: Option<String>,
    pub refund_type: storage_enums::RefundType,
    pub total_amount: i64,
    pub currency: storage_enums::Currency,
    pub refund_amount: i64,
    pub refund_status: storage_enums::RefundStatus,
    pub sent_to_gateway: bool,
    pub refund_error_message: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub refund_arn: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub updated_at: PrimitiveDateTime,
    pub description: Option<String>,
    pub attempt_id: String,
    pub refund_reason: Option<String>,
    pub refund_error_code: Option<String>,
    pub profile_id: Option<String>,
    pub updated_by: String,
    pub merchant_connector_id: Option<String>,
}

#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    Insertable,
    router_derive::DebugAsDisplay,
    serde::Serialize,
    serde::Deserialize,
    router_derive::Setter,
)]
#[diesel(table_name = refund)]
pub struct RefundNew {
    pub refund_id: String,
    pub payment_id: String,
    pub merchant_id: String,
    pub internal_reference_id: String,
    pub external_reference_id: Option<String>,
    pub connector_transaction_id: String,
    pub connector: String,
    pub connector_refund_id: Option<String>,
    pub refund_type: storage_enums::RefundType,
    pub total_amount: i64,
    pub currency: storage_enums::Currency,
    pub refund_amount: i64,
    pub refund_status: storage_enums::RefundStatus,
    pub sent_to_gateway: bool,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub refund_arn: Option<String>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created_at: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub modified_at: Option<PrimitiveDateTime>,
    pub description: Option<String>,
    pub attempt_id: String,
    pub refund_reason: Option<String>,
    pub profile_id: Option<String>,
    pub updated_by: String,
    pub merchant_connector_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RefundUpdate {
    Update {
        connector_refund_id: String,
        refund_status: storage_enums::RefundStatus,
        sent_to_gateway: bool,
        refund_error_message: Option<String>,
        refund_arn: String,
        updated_by: String,
    },
    MetadataAndReasonUpdate {
        metadata: Option<pii::SecretSerdeValue>,
        reason: Option<String>,
        updated_by: String,
    },
    StatusUpdate {
        connector_refund_id: Option<String>,
        sent_to_gateway: bool,
        refund_status: storage_enums::RefundStatus,
        updated_by: String,
    },
    ErrorUpdate {
        refund_status: Option<storage_enums::RefundStatus>,
        refund_error_message: Option<String>,
        refund_error_code: Option<String>,
        updated_by: String,
    },
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = refund)]
pub struct RefundUpdateInternal {
    connector_refund_id: Option<String>,
    refund_status: Option<storage_enums::RefundStatus>,
    sent_to_gateway: Option<bool>,
    refund_error_message: Option<String>,
    refund_arn: Option<String>,
    metadata: Option<pii::SecretSerdeValue>,
    refund_reason: Option<String>,
    refund_error_code: Option<String>,
    updated_by: String,
}

impl RefundUpdateInternal {
    pub fn create_refund(self, source: Refund) -> Refund {
        Refund {
            connector_refund_id: self.connector_refund_id,
            refund_status: self.refund_status.unwrap_or_default(),
            sent_to_gateway: self.sent_to_gateway.unwrap_or_default(),
            refund_error_message: self.refund_error_message,
            refund_arn: self.refund_arn,
            metadata: self.metadata,
            refund_reason: self.refund_reason,
            refund_error_code: self.refund_error_code,
            updated_by: self.updated_by,
            ..source
        }
    }
}

impl From<RefundUpdate> for RefundUpdateInternal {
    fn from(refund_update: RefundUpdate) -> Self {
        match refund_update {
            RefundUpdate::Update {
                connector_refund_id,
                refund_status,
                sent_to_gateway,
                refund_error_message,
                refund_arn,
                updated_by,
            } => Self {
                connector_refund_id: Some(connector_refund_id),
                refund_status: Some(refund_status),
                sent_to_gateway: Some(sent_to_gateway),
                refund_error_message,
                refund_arn: Some(refund_arn),
                updated_by,
                ..Default::default()
            },
            RefundUpdate::MetadataAndReasonUpdate {
                metadata,
                reason,
                updated_by,
            } => Self {
                metadata,
                refund_reason: reason,
                updated_by,
                ..Default::default()
            },
            RefundUpdate::StatusUpdate {
                connector_refund_id,
                sent_to_gateway,
                refund_status,
                updated_by,
            } => Self {
                connector_refund_id,
                sent_to_gateway: Some(sent_to_gateway),
                refund_status: Some(refund_status),
                updated_by,
                ..Default::default()
            },
            RefundUpdate::ErrorUpdate {
                refund_status,
                refund_error_message,
                refund_error_code,
                updated_by,
            } => Self {
                refund_status,
                refund_error_message,
                refund_error_code,
                updated_by,
                ..Default::default()
            },
        }
    }
}

impl RefundUpdate {
    pub fn apply_changeset(self, source: Refund) -> Refund {
        let pa_update: RefundUpdateInternal = self.into();
        Refund {
            connector_refund_id: pa_update.connector_refund_id.or(source.connector_refund_id),
            refund_status: pa_update.refund_status.unwrap_or(source.refund_status),
            sent_to_gateway: pa_update.sent_to_gateway.unwrap_or(source.sent_to_gateway),
            refund_error_message: pa_update
                .refund_error_message
                .or(source.refund_error_message),
            refund_error_code: pa_update.refund_error_code.or(source.refund_error_code),
            refund_arn: pa_update.refund_arn.or(source.refund_arn),
            metadata: pa_update.metadata.or(source.metadata),
            refund_reason: pa_update.refund_reason.or(source.refund_reason),
            updated_by: pa_update.updated_by,
            ..source
        }
    }
}

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct RefundCoreWorkflow {
    pub refund_internal_reference_id: String,
    pub connector_transaction_id: String,
    pub merchant_id: String,
    pub payment_id: String,
}
