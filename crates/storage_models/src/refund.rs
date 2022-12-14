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
    pub transaction_id: String,
    pub connector: String,
    pub pg_refund_id: Option<String>,
    pub external_reference_id: Option<String>,
    pub refund_type: storage_enums::RefundType,
    pub total_amount: i32,
    pub currency: storage_enums::Currency,
    pub refund_amount: i32,
    pub refund_status: storage_enums::RefundStatus,
    pub sent_to_gateway: bool,
    pub refund_error_message: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub refund_arn: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = refund)]
pub struct RefundNew {
    pub refund_id: String,
    pub payment_id: String,
    pub merchant_id: String,
    pub internal_reference_id: String,
    pub external_reference_id: Option<String>,
    pub transaction_id: String,
    pub connector: String,
    pub pg_refund_id: Option<String>,
    pub refund_type: storage_enums::RefundType,
    pub total_amount: i32,
    pub currency: storage_enums::Currency,
    pub refund_amount: i32,
    pub refund_status: storage_enums::RefundStatus,
    pub sent_to_gateway: bool,
    pub refund_error_message: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub refund_arn: Option<String>,
    pub created_at: Option<PrimitiveDateTime>,
    pub modified_at: Option<PrimitiveDateTime>,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub enum RefundUpdate {
    Update {
        pg_refund_id: String,
        refund_status: storage_enums::RefundStatus,
        sent_to_gateway: bool,
        refund_error_message: Option<String>,
        refund_arn: String,
    },
    MetadataUpdate {
        metadata: Option<serde_json::Value>,
    },
    StatusUpdate {
        pg_refund_id: Option<String>,
        sent_to_gateway: bool,
        refund_status: storage_enums::RefundStatus,
    },
    ErrorUpdate {
        refund_status: Option<storage_enums::RefundStatus>,
        refund_error_message: Option<String>,
    },
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = refund)]
pub struct RefundUpdateInternal {
    pg_refund_id: Option<String>,
    refund_status: Option<storage_enums::RefundStatus>,
    sent_to_gateway: Option<bool>,
    refund_error_message: Option<String>,
    refund_arn: Option<String>,
    metadata: Option<serde_json::Value>,
}

impl From<RefundUpdate> for RefundUpdateInternal {
    fn from(refund_update: RefundUpdate) -> Self {
        match refund_update {
            RefundUpdate::Update {
                pg_refund_id,
                refund_status,
                sent_to_gateway,
                refund_error_message,
                refund_arn,
            } => Self {
                pg_refund_id: Some(pg_refund_id),
                refund_status: Some(refund_status),
                sent_to_gateway: Some(sent_to_gateway),
                refund_error_message,
                refund_arn: Some(refund_arn),
                ..Default::default()
            },
            RefundUpdate::MetadataUpdate { metadata } => Self {
                metadata,
                ..Default::default()
            },
            RefundUpdate::StatusUpdate {
                pg_refund_id,
                sent_to_gateway,
                refund_status,
            } => Self {
                pg_refund_id,
                sent_to_gateway: Some(sent_to_gateway),
                refund_status: Some(refund_status),
                ..Default::default()
            },
            RefundUpdate::ErrorUpdate {
                refund_status,
                refund_error_message,
            } => Self {
                refund_status,
                refund_error_message,
                ..Default::default()
            },
        }
    }
}

impl RefundUpdate {
    pub fn apply_changeset(self, source: Refund) -> Refund {
        let pa_update: RefundUpdateInternal = self.into();
        Refund {
            pg_refund_id: pa_update.pg_refund_id,
            refund_status: pa_update.refund_status.unwrap_or(source.refund_status),
            sent_to_gateway: pa_update.sent_to_gateway.unwrap_or(source.sent_to_gateway),
            refund_error_message: pa_update.refund_error_message,
            refund_arn: pa_update.refund_arn,
            metadata: pa_update.metadata,
            ..source
        }
    }
}

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct RefundCoreWorkflow {
    pub refund_internal_reference_id: String,
    pub transaction_id: String,
    pub merchant_id: String,
    pub payment_id: String,
}
