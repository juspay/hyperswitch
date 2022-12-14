use std::{convert::From, default::Default};

use serde::{Deserialize, Serialize};

use crate::types::api::refunds::{RefundRequest, RefundResponse, RefundStatus};

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct StripeCreateRefundRequest {
    pub(crate) amount: Option<i64>,
    pub(crate) payment_intent: String,
    pub(crate) reason: Option<String>,
}

#[derive(Clone, Serialize, PartialEq, Eq)]
pub(crate) struct StripeCreateRefundResponse {
    pub(crate) id: String,
    pub(crate) amount: i64,
    pub(crate) currency: String,
    pub(crate) payment_intent: String,
    pub(crate) status: StripeRefundStatus,
}

#[derive(Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StripeRefundStatus {
    Succeeded,
    Failed,
    Pending,
    RequiresAction,
}

impl From<StripeCreateRefundRequest> for RefundRequest {
    fn from(req: StripeCreateRefundRequest) -> Self {
        Self {
            amount: req.amount,
            payment_id: req.payment_intent,
            reason: req.reason,
            ..Default::default()
        }
    }
}

impl From<RefundStatus> for StripeRefundStatus {
    fn from(status: RefundStatus) -> Self {
        match status {
            RefundStatus::Succeeded => Self::Succeeded,
            RefundStatus::Failed => Self::Failed,
            RefundStatus::Pending => Self::Pending,
            RefundStatus::Review => Self::RequiresAction,
        }
    }
}

impl From<RefundResponse> for StripeCreateRefundResponse {
    fn from(res: RefundResponse) -> Self {
        Self {
            id: res.refund_id,
            amount: res.amount,
            currency: res.currency.to_ascii_lowercase(),
            payment_intent: res.payment_id,
            status: res.status.into(),
        }
    }
}
