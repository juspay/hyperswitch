use serde::{Deserialize, Serialize};

use crate::enums;

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefundRequest {
    pub refund_id: Option<String>,
    pub payment_id: String,
    pub merchant_id: Option<String>,
    pub amount: Option<i64>,
    pub reason: Option<String>,
    pub refund_type: Option<RefundType>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefundType {
    #[default]
    Scheduled,
    Instant,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct RefundResponse {
    pub refund_id: String,
    pub payment_id: String,
    pub amount: i64,
    pub currency: String,
    pub reason: Option<String>,
    pub status: RefundStatus,
    pub metadata: Option<serde_json::Value>,
    pub error_message: Option<String>,
}

#[derive(Debug, Eq, Clone, PartialEq, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Pending,
    Review,
}

impl From<enums::RefundStatus> for RefundStatus {
    fn from(status: enums::RefundStatus) -> Self {
        match status {
            enums::RefundStatus::Failure | enums::RefundStatus::TransactionFailure => Self::Failed,
            enums::RefundStatus::ManualReview => Self::Review,
            enums::RefundStatus::Pending => Self::Pending,
            enums::RefundStatus::Success => Self::Succeeded,
        }
    }
}
