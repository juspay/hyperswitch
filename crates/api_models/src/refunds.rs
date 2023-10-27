use common_utils::pii;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

use crate::{admin, enums};

#[derive(Default, Debug, ToSchema, Clone, Deserialize, Serialize)]
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
    pub metadata: Option<pii::SecretSerdeValue>,

    /// Merchant connector details used to make payments.
    #[schema(value_type = Option<MerchantConnectorDetailsWrap>)]
    pub merchant_connector_details: Option<admin::MerchantConnectorDetailsWrap>,
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct RefundsRetrieveBody {
    pub force_sync: Option<bool>,
}

#[derive(Default, Debug, ToSchema, Clone, Deserialize, Serialize)]
pub struct RefundsRetrieveRequest {
    /// Unique Identifier for the Refund. This is to ensure idempotency for multiple partial refund initiated against the same payment. If the identifiers is not defined by the merchant, this filed shall be auto generated and provide in the API response. It is recommended to generate uuid(v4) as the refund_id.
    #[schema(
        max_length = 30,
        min_length = 30,
        example = "ref_mbabizu24mvu3mela5njyhpit4"
    )]
    pub refund_id: String,

    /// `force_sync` with the connector to get refund details
    /// (defaults to false)
    pub force_sync: Option<bool>,

    /// Merchant connector details used to make payments.
    pub merchant_connector_details: Option<admin::MerchantConnectorDetailsWrap>,
}

#[derive(Default, Debug, ToSchema, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefundUpdateRequest {
    /// An arbitrary string attached to the object. Often useful for displaying to users and your customer support executive
    #[schema(max_length = 255, example = "Customer returned the product")]
    pub reason: Option<String>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type  = Option<Object>, example = r#"{ "city": "NY", "unit": "245" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,
}

#[derive(
    Default, Debug, Clone, Copy, ToSchema, Deserialize, Serialize, Eq, PartialEq, strum::Display,
)]
#[serde(rename_all = "snake_case")]
pub enum RefundType {
    #[default]
    Scheduled,
    Instant,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct RefundResponse {
    /// The identifier for refund
    pub refund_id: String,
    /// The identifier for payment
    pub payment_id: String,
    /// The refund amount, which should be less than or equal to the total payment amount. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc
    pub amount: i64,
    /// The three-letter ISO currency code
    pub currency: String,
    /// An arbitrary string attached to the object. Often useful for displaying to users and your customer support executive
    pub reason: Option<String>,
    /// The status for refund
    pub status: RefundStatus,
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object
    #[schema(value_type = Option<Object>)]
    pub metadata: Option<pii::SecretSerdeValue>,
    /// The error message
    pub error_message: Option<String>,
    /// The code for the error
    pub error_code: Option<String>,
    /// The timestamp at which refund is created
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub created_at: Option<PrimitiveDateTime>,
    /// The timestamp at which refund is updated
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub updated_at: Option<PrimitiveDateTime>,
    /// The connector used for the refund and the corresponding payment
    #[schema(example = "stripe")]
    pub connector: String,
    pub profile_id: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct RefundListRequest {
    /// The identifier for the payment
    pub payment_id: Option<String>,
    /// The identifier for the refund
    pub refund_id: Option<String>,
    /// The identifier for business profile
    pub profile_id: Option<String>,
    /// Limit on the number of objects to return
    pub limit: Option<i64>,
    /// The starting point within a list of objects
    pub offset: Option<i64>,
    /// The time range for which objects are needed. TimeRange has two fields start_time and end_time from which objects can be filtered as per required scenarios (created_at, time less than, greater than etc).
    #[serde(flatten)]
    pub time_range: Option<TimeRange>,
    /// The list of connectors to filter refunds list
    pub connector: Option<Vec<String>>,
    /// The list of currencies to filter refunds list
    #[schema(value_type = Option<Vec<Currency>>)]
    pub currency: Option<Vec<enums::Currency>>,
    /// The list of refund statuses to filter refunds list
    #[schema(value_type = Option<Vec<RefundStatus>>)]
    pub refund_status: Option<Vec<enums::RefundStatus>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct TimeRange {
    /// The start time to filter refunds list or to get list of filters. To get list of filters start time is needed to be passed
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub start_time: PrimitiveDateTime,
    /// The end time to filter refunds list or to get list of filters. If not passed the default time is now
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub end_time: Option<PrimitiveDateTime>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct RefundListResponse {
    /// The number of refunds included in the list
    pub count: usize,
    /// The total number of refunds in the list
    pub total_count: i64,
    /// The List of refund response object
    pub data: Vec<RefundResponse>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, ToSchema)]
pub struct RefundListMetaData {
    /// The list of available connector filters
    pub connector: Vec<String>,
    /// The list of available currency filters
    #[schema(value_type = Vec<Currency>)]
    pub currency: Vec<enums::Currency>,
    /// The list of available refund status filters
    #[schema(value_type = Vec<RefundStatus>)]
    pub status: Vec<enums::RefundStatus>,
}

/// The status for refunds
#[derive(
    Debug,
    Eq,
    Clone,
    Copy,
    PartialEq,
    Default,
    Deserialize,
    Serialize,
    ToSchema,
    strum::Display,
    strum::EnumIter,
)]
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
