use std::{convert::From, default::Default};

use serde::{Deserialize, Serialize};

use crate::{core::errors, types::api::refunds};

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct StripeCreateRefundRequest {
    pub amount: Option<i64>,
    pub payment_intent: String,
    pub reason: Option<String>,
}

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct StripeUpdateRefundRequest {
    pub metadata: Option<serde_json::Value>,
}

#[derive(Clone, Serialize, PartialEq, Eq)]
pub struct StripeCreateRefundResponse {
    pub id: String,
    pub amount: i64,
    pub currency: String,
    pub payment_intent: String,
    pub status: StripeRefundStatus,
    pub created: Option<i64>,
    pub metadata: serde_json::Value,
}

#[derive(Clone, Serialize, Deserialize, Eq, PartialEq)]
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
            amount: req.amount,
            payment_id: req.payment_intent,
            reason: req.reason,
            refund_type: Some(refunds::RefundType::Instant),
            ..Default::default()
        }
    }
}

impl From<StripeUpdateRefundRequest> for refunds::RefundUpdateRequest {
    fn from(req: StripeUpdateRefundRequest) -> Self {
        Self {
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

impl TryFrom<refunds::RefundResponse> for StripeCreateRefundResponse {
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn try_from(res: refunds::RefundResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            id: res.refund_id,
            amount: res.amount,
            currency: res.currency.to_ascii_lowercase(),
            payment_intent: res.payment_id,
            status: res.status.into(),
            created: res.created_at.map(|t| t.assume_utc().unix_timestamp()),
            metadata: res.metadata.unwrap_or_else(|| serde_json::json!({})),
        })
    }
}
