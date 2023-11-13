use std::{convert::From, default::Default};

use common_utils::pii;
use serde::{Deserialize, Serialize};

use crate::types::api::{admin, refunds};

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StripeCreateRefundRequest {
    pub refund_id: Option<String>,
    pub amount: Option<i64>,
    pub payment_intent: String,
    pub reason: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub merchant_connector_details: Option<admin::MerchantConnectorDetailsWrap>,
}

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct StripeUpdateRefundRequest {
    #[serde(skip)]
    pub refund_id: String,
    pub metadata: Option<pii::SecretSerdeValue>,
}

#[derive(Clone, Serialize, PartialEq, Eq, Debug)]
pub struct StripeRefundResponse {
    pub id: String,
    pub amount: i64,
    pub currency: String,
    pub payment_intent: String,
    pub status: StripeRefundStatus,
    pub created: Option<i64>,
    pub metadata: pii::SecretSerdeValue,
}

#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum StripeRefundStatus {
    Succeeded,
    Failed,
    Pending,
    RequiresAction,
}

impl From<StripeCreateRefundRequest> for refunds::RefundRequest {
    fn from(req: StripeCreateRefundRequest) -> Self {
        Self {
            refund_id: req.refund_id,
            amount: req.amount,
            payment_id: req.payment_intent,
            reason: req.reason,
            refund_type: Some(refunds::RefundType::Instant),
            metadata: req.metadata,
            merchant_connector_details: req.merchant_connector_details,
            ..Default::default()
        }
    }
}

impl From<StripeUpdateRefundRequest> for refunds::RefundUpdateRequest {
    fn from(req: StripeUpdateRefundRequest) -> Self {
        Self {
            refund_id: req.refund_id,
            metadata: req.metadata,
            reason: None,
        }
    }
}

impl From<refunds::RefundStatus> for StripeRefundStatus {
    fn from(status: refunds::RefundStatus) -> Self {
        match status {
            refunds::RefundStatus::Succeeded => Self::Succeeded,
            refunds::RefundStatus::Failed => Self::Failed,
            refunds::RefundStatus::Pending => Self::Pending,
            refunds::RefundStatus::Review => Self::RequiresAction,
        }
    }
}

impl From<refunds::RefundResponse> for StripeRefundResponse {
    fn from(res: refunds::RefundResponse) -> Self {
        Self {
            id: res.refund_id,
            amount: res.amount,
            currency: res.currency.to_ascii_lowercase(),
            payment_intent: res.payment_id,
            status: res.status.into(),
            created: res.created_at.map(|t| t.assume_utc().unix_timestamp()),
            metadata: res
                .metadata
                .unwrap_or_else(|| masking::Secret::new(serde_json::json!({}))),
        }
    }
}
