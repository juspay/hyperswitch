use common_utils::custom_serde;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

use crate::enums;

#[derive(Default, Debug, ToSchema, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefundRequest {
    /// Unique Identifier for the Refund. This is to ensure idempotency for multiple partial refund initiated against the same payment. If the identifiers is not defined by the merchant, this filed shall be auto generated and provide in the API response. It is recommended to generate uuid(v4) as the refund_id.
    #[schema(
        max_length = 30,
        min_length = 30,
        example = "ref_mbabizu24mvu3mela5njyhpit4"
    )]
    pub refund_id: Option<String>,

    /// Total amount for which the refund is to be initiated. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc. If not provided, this will default to the full payment amount
    #[schema(
        max_length = 30,
        min_length = 30,
        example = "pay_mbabizu24mvu3mela5njyhpit4"
    )]
    pub payment_id: String,

    /// The identifier for the Merchant Account
    #[schema(max_length = 255, example = "y3oqhf46pyzuxjbcn2giaqnb44")]
    pub merchant_id: Option<String>,

    /// Total amount for which the refund is to be initiated. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc., If not provided, this will default to the full payment amount
    #[schema(minimum = 100, example = 6540)]
    pub amount: Option<i64>,

    /// An arbitrary string attached to the object. Often useful for displaying to users and your customer support executive
    #[schema(max_length = 255, example = "Customer returned the product")]
    pub reason: Option<String>,

    /// The type of refund based on waiting time for processing: Scheduled or Instant Refund
    #[schema(default = "Instant", example = "Instant")]
    pub refund_type: Option<RefundType>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type  = Option<Object>, example = r#"{ "city": "NY", "unit": "245" }"#)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Default, Debug, ToSchema, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefundUpdateRequest {
    /// An arbitrary string attached to the object. Often useful for displaying to users and your customer support executive
    #[schema(max_length = 255, example = "Customer returned the product")]
    pub reason: Option<String>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type  = Option<Object>, example = r#"{ "city": "NY", "unit": "245" }"#)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Default, Debug, Clone, ToSchema, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefundType {
    #[default]
    Scheduled,
    Instant,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct RefundResponse {
    pub refund_id: String,
    pub payment_id: String,
    pub amount: i64,
    pub currency: String,
    pub reason: Option<String>,
    pub status: RefundStatus,
    #[schema(value_type = Option<Object>)]
    pub metadata: Option<serde_json::Value>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct RefundListRequest {
    pub payment_id: Option<String>,
    pub limit: Option<i64>,
    #[serde(default, with = "custom_serde::iso8601::option")]
    pub created: Option<PrimitiveDateTime>,
    #[serde(default, rename = "created.lt", with = "custom_serde::iso8601::option")]
    pub created_lt: Option<PrimitiveDateTime>,
    #[serde(default, rename = "created.gt", with = "custom_serde::iso8601::option")]
    pub created_gt: Option<PrimitiveDateTime>,
    #[serde(
        default,
        rename = "created.lte",
        with = "custom_serde::iso8601::option"
    )]
    pub created_lte: Option<PrimitiveDateTime>,
    #[serde(
        default,
        rename = "created.gte",
        with = "custom_serde::iso8601::option"
    )]
    pub created_gte: Option<PrimitiveDateTime>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct RefundListResponse {
    pub data: Vec<RefundResponse>,
}

#[derive(Debug, Eq, Clone, PartialEq, Default, Deserialize, Serialize, ToSchema)]
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
