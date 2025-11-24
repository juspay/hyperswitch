use std::collections::HashMap;

pub use common_utils::types::MinorUnit;
use common_utils::{pii, types::TimeRange};
use serde::{Deserialize, Serialize};
use smithy::SmithyModel;
use time::PrimitiveDateTime;
use utoipa::ToSchema;

use super::payments::AmountFilter;
#[cfg(feature = "v1")]
use crate::admin;
use crate::{admin::MerchantConnectorInfo, enums};

#[cfg(feature = "v1")]
#[derive(Default, Debug, ToSchema, Clone, Deserialize, Serialize, SmithyModel)]
#[serde(deny_unknown_fields)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct RefundRequest {
    /// The payment id against which refund is to be initiated
    #[schema(
        max_length = 30,
        min_length = 30,
        example = "pay_mbabizu24mvu3mela5njyhpit4",
        value_type = String,
    )]
    #[smithy(value_type = "String")]
    pub payment_id: common_utils::id_type::PaymentId,

    /// Unique Identifier for the Refund. This is to ensure idempotency for multiple partial refunds initiated against the same payment. If this is not passed by the merchant, this field shall be auto generated and provided in the API response. It is recommended to generate uuid(v4) as the refund_id.
    #[schema(
        max_length = 30,
        min_length = 30,
        example = "ref_mbabizu24mvu3mela5njyhpit4"
    )]
    #[smithy(value_type = "Option<String>")]
    pub refund_id: Option<String>,

    /// The identifier for the Merchant Account
    #[schema(max_length = 255, example = "y3oqhf46pyzuxjbcn2giaqnb44", value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub merchant_id: Option<common_utils::id_type::MerchantId>,

    /// Total amount for which the refund is to be initiated. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc., If not provided, this will default to the full payment amount
    #[schema(value_type = Option<i64> , minimum = 100, example = 6540)]
    #[smithy(value_type = "Option<i64>")]
    pub amount: Option<MinorUnit>,

    /// Reason for the refund. Often useful for displaying to users and your customer support executive. In case the payment went through Stripe, this field needs to be passed with one of these enums: `duplicate`, `fraudulent`, or `requested_by_customer`
    #[schema(max_length = 255, example = "Customer returned the product")]
    #[smithy(value_type = "Option<String>")]
    pub reason: Option<String>,

    /// To indicate whether to refund needs to be instant or scheduled. Default value is instant
    #[schema(default = "Instant", example = "Instant")]
    #[smithy(value_type = "Option<RefundType>")]
    pub refund_type: Option<RefundType>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type  = Option<Object>, example = r#"{ "city": "NY", "unit": "245" }"#)]
    #[smithy(value_type = "Option<Object>")]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// Merchant connector details used to make payments.
    #[schema(value_type = Option<MerchantConnectorDetailsWrap>)]
    #[smithy(value_type = "Option<MerchantConnectorDetailsWrap>")]
    pub merchant_connector_details: Option<admin::MerchantConnectorDetailsWrap>,

    /// Charge specific fields for controlling the revert of funds from either platform or connected account
    #[schema(value_type = Option<SplitRefund>)]
    #[smithy(value_type = "Option<SplitRefund>")]
    pub split_refunds: Option<common_types::refunds::SplitRefund>,

    /// If true, returns stringified connector raw response body
    pub all_keys_required: Option<bool>,
}

#[cfg(feature = "v2")]
#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefundsCreateRequest {
    /// The payment id against which refund is initiated
    #[schema(
        max_length = 30,
        min_length = 30,
        example = "pay_mbabizu24mvu3mela5njyhpit4",
        value_type = String,
    )]
    pub payment_id: common_utils::id_type::GlobalPaymentId,

    /// Unique Identifier for the Refund given by the Merchant.
    #[schema(
        max_length = 64,
        min_length = 1,
        example = "ref_mbabizu24mvu3mela5njyhpit4",
        value_type = String,
    )]
    pub merchant_reference_id: common_utils::id_type::RefundReferenceId,

    /// The identifier for the Merchant Account
    #[schema(max_length = 255, example = "y3oqhf46pyzuxjbcn2giaqnb44", value_type = Option<String>)]
    pub merchant_id: Option<common_utils::id_type::MerchantId>,

    /// Total amount for which the refund is to be initiated. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc., If not provided, this will default to the amount_captured of the payment
    #[schema(value_type = Option<i64> , minimum = 100, example = 6540)]
    pub amount: Option<MinorUnit>,

    /// Reason for the refund. Often useful for displaying to users and your customer support executive.
    #[schema(max_length = 255, example = "Customer returned the product")]
    pub reason: Option<String>,

    /// To indicate whether to refund needs to be instant or scheduled. Default value is instant
    #[schema(default = "Instant", example = "Instant")]
    pub refund_type: Option<RefundType>,

    /// Metadata is useful for storing additional, unstructured information on an object.
    #[schema(value_type  = Option<Object>, example = r#"{ "city": "NY", "unit": "245" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// Merchant connector details used to make payments.
    #[schema(value_type = Option<MerchantConnectorAuthDetails>)]
    pub merchant_connector_details: Option<common_types::domain::MerchantConnectorAuthDetails>,

    /// If true, returns stringified connector raw response body
    pub return_raw_connector_response: Option<bool>,
}

#[cfg(feature = "v1")]
#[derive(Default, Debug, Clone, Deserialize)]
pub struct RefundsRetrieveBody {
    pub force_sync: Option<bool>,
    pub all_keys_required: Option<bool>,
}

#[cfg(feature = "v2")]
#[derive(Default, Debug, Clone, Deserialize)]
pub struct RefundsRetrieveBody {
    pub force_sync: Option<bool>,
    pub return_raw_connector_response: Option<bool>,
}

#[cfg(feature = "v2")]
#[derive(Default, Debug, Clone, Deserialize)]
pub struct RefundsRetrievePayload {
    /// `force_sync` with the connector to get refund details
    pub force_sync: Option<bool>,

    /// Merchant connector details used to make payments.
    pub merchant_connector_details: Option<common_types::domain::MerchantConnectorAuthDetails>,

    /// If true, returns stringified connector raw response body
    pub return_raw_connector_response: Option<bool>,
}

#[cfg(feature = "v1")]
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

    /// If true, returns stringified connector raw response body
    pub all_keys_required: Option<bool>,
}

#[cfg(feature = "v2")]
#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
pub struct RefundsRetrieveRequest {
    /// Unique Identifier for the Refund. This is to ensure idempotency for multiple partial refund initiated against the same payment. If the identifiers is not defined by the merchant, this filed shall be auto generated and provide in the API response. It is recommended to generate uuid(v4) as the refund_id.
    #[schema(
        max_length = 30,
        min_length = 30,
        example = "ref_mbabizu24mvu3mela5njyhpit4"
    )]
    pub refund_id: common_utils::id_type::GlobalRefundId,

    /// `force_sync` with the connector to get refund details
    /// (defaults to false)
    pub force_sync: Option<bool>,

    /// Merchant connector details used to make payments.
    #[schema(value_type = Option<MerchantConnectorAuthDetails>)]
    pub merchant_connector_details: Option<common_types::domain::MerchantConnectorAuthDetails>,

    /// If true, returns stringified connector raw response body
    pub return_raw_connector_response: Option<bool>,
}

#[derive(Default, Debug, ToSchema, Clone, Deserialize, Serialize, SmithyModel)]
#[serde(deny_unknown_fields)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct RefundUpdateRequest {
    #[serde(skip)]
    pub refund_id: String,
    /// An arbitrary string attached to the object. Often useful for displaying to users and your customer support executive
    #[schema(max_length = 255, example = "Customer returned the product")]
    #[smithy(value_type = "Option<String>")]
    pub reason: Option<String>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type  = Option<Object>, example = r#"{ "city": "NY", "unit": "245" }"#)]
    #[smithy(value_type = "Option<Object>")]
    pub metadata: Option<pii::SecretSerdeValue>,
}

#[cfg(feature = "v2")]
#[derive(Default, Debug, ToSchema, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefundMetadataUpdateRequest {
    /// An arbitrary string attached to the object. Often useful for displaying to users and your customer support executive
    #[schema(max_length = 255, example = "Customer returned the product")]
    pub reason: Option<String>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type  = Option<Object>, example = r#"{ "city": "NY", "unit": "245" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,
}

#[derive(Default, Debug, ToSchema, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefundManualUpdateRequest {
    #[serde(skip)]
    pub refund_id: String,
    /// Merchant ID
    #[schema(value_type = String)]
    pub merchant_id: common_utils::id_type::MerchantId,
    /// The status for refund
    pub status: Option<RefundStatus>,
    /// The code for the error
    pub error_code: Option<String>,
    /// The error message
    pub error_message: Option<String>,
}

#[cfg(feature = "v1")]
/// To indicate whether to refund needs to be instant or scheduled
#[derive(
    Default,
    Debug,
    Clone,
    Copy,
    ToSchema,
    Deserialize,
    Serialize,
    Eq,
    PartialEq,
    strum::Display,
    SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum RefundType {
    Scheduled,
    #[default]
    Instant,
}

#[cfg(feature = "v2")]
/// To indicate whether the refund needs to be instant or scheduled
#[derive(
    Default, Debug, Clone, Copy, ToSchema, Deserialize, Serialize, Eq, PartialEq, strum::Display,
)]
#[serde(rename_all = "snake_case")]
pub enum RefundType {
    Scheduled,
    #[default]
    Instant,
}

#[cfg(feature = "v1")]
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize, ToSchema, SmithyModel)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct RefundResponse {
    /// Unique Identifier for the refund
    #[smithy(value_type = "String")]
    pub refund_id: String,
    /// The payment id against which refund is initiated
    #[schema(value_type = String)]
    #[smithy(value_type = "String")]
    pub payment_id: common_utils::id_type::PaymentId,
    /// The refund amount, which should be less than or equal to the total payment amount. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc
    #[schema(value_type = i64 , minimum = 100, example = 6540)]
    #[smithy(value_type = "i64")]
    pub amount: MinorUnit,
    /// The three-letter ISO currency code
    #[smithy(value_type = "String")]
    pub currency: String,
    /// The status for refund
    #[smithy(value_type = "RefundStatus")]
    pub status: RefundStatus,
    /// An arbitrary string attached to the object. Often useful for displaying to users and your customer support executive
    #[smithy(value_type = "Option<String>")]
    pub reason: Option<String>,
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object
    #[schema(value_type = Option<Object>)]
    #[smithy(value_type = "Option<Object>")]
    pub metadata: Option<pii::SecretSerdeValue>,
    /// The error message
    #[smithy(value_type = "Option<String>")]
    pub error_message: Option<String>,
    /// The code for the error
    #[smithy(value_type = "Option<String>")]
    pub error_code: Option<String>,
    /// Error code unified across the connectors is received here if there was an error while calling connector
    #[smithy(value_type = "Option<String>")]
    pub unified_code: Option<String>,
    /// Error message unified across the connectors is received here if there was an error while calling connector
    #[smithy(value_type = "Option<String>")]
    pub unified_message: Option<String>,
    /// The timestamp at which refund is created
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    #[smithy(value_type = "Option<String>")]
    pub created_at: Option<PrimitiveDateTime>,
    /// The timestamp at which refund is updated
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    #[smithy(value_type = "Option<String>")]
    pub updated_at: Option<PrimitiveDateTime>,
    /// The connector used for the refund and the corresponding payment
    #[schema(example = "stripe")]
    #[smithy(value_type = "String")]
    pub connector: String,
    /// The id of business profile for this refund
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub profile_id: Option<common_utils::id_type::ProfileId>,
    /// The merchant_connector_id of the processor through which this payment went through
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    /// Charge specific fields for controlling the revert of funds from either platform or connected account
    #[schema(value_type = Option<SplitRefund>,)]
    #[smithy(value_type = "Option<SplitRefund>")]
    pub split_refunds: Option<common_types::refunds::SplitRefund>,
    /// Error code received from the issuer in case of failed refunds
    #[smithy(value_type = "Option<String>")]
    pub issuer_error_code: Option<String>,
    /// Error message received from the issuer in case of failed refunds
    #[smithy(value_type = "Option<String>")]
    pub issuer_error_message: Option<String>,
    /// Contains whole connector response
    #[schema(value_type = Option<String>)]
    pub raw_connector_response: Option<masking::Secret<String>>,
}

#[cfg(feature = "v1")]
impl RefundResponse {
    pub fn get_refund_id_as_string(&self) -> String {
        self.refund_id.clone()
    }
}

#[cfg(feature = "v2")]
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct RefundResponse {
    /// Global Refund Id for the refund
    #[schema(value_type = String)]
    pub id: common_utils::id_type::GlobalRefundId,
    /// The payment id against which refund is initiated
    #[schema(value_type = String)]
    pub payment_id: common_utils::id_type::GlobalPaymentId,
    /// Unique Identifier for the Refund. This is to ensure idempotency for multiple partial refunds initiated against the same payment.
    #[schema(
        max_length = 30,
        min_length = 30,
        example = "ref_mbabizu24mvu3mela5njyhpit4",
        value_type = Option<String>,
    )]
    pub merchant_reference_id: Option<common_utils::id_type::RefundReferenceId>,
    /// The refund amount
    #[schema(value_type = i64 , minimum = 100, example = 6540)]
    pub amount: MinorUnit,
    /// The three-letter ISO currency code
    #[schema(value_type = Currency)]
    pub currency: common_enums::Currency,
    /// The status for refund
    pub status: RefundStatus,
    /// An arbitrary string attached to the object
    pub reason: Option<String>,
    /// Metadata is useful for storing additional, unstructured information on an object
    #[schema(value_type = Option<Object>)]
    pub metadata: Option<pii::SecretSerdeValue>,
    /// The error details for the refund
    pub error_details: Option<RefundErrorDetails>,
    /// The timestamp at which refund is created
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    /// The timestamp at which refund is updated
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub updated_at: PrimitiveDateTime,
    /// The connector used for the refund and the corresponding payment
    #[schema(example = "stripe", value_type = Connector)]
    pub connector: enums::Connector,
    /// The id of business profile for this refund
    #[schema(value_type = String)]
    pub profile_id: common_utils::id_type::ProfileId,
    /// The merchant_connector_id of the processor through which this payment went through
    #[schema(value_type = String)]
    pub merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    /// The reference id of the connector for the refund
    pub connector_refund_reference_id: Option<String>,
    /// Contains raw connector response
    #[schema(value_type = Option<String>)]
    pub raw_connector_response: Option<masking::Secret<String>>,
}

#[cfg(feature = "v2")]
impl RefundResponse {
    pub fn get_refund_id_as_string(&self) -> String {
        self.id.get_string_repr().to_owned()
    }
}

#[cfg(feature = "v2")]
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct RefundErrorDetails {
    pub code: String,
    pub message: String,
}

#[cfg(feature = "v1")]
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct RefundListRequest {
    /// The identifier for the payment
    #[schema(value_type = Option<String>)]
    pub payment_id: Option<common_utils::id_type::PaymentId>,
    /// The identifier for the refund
    pub refund_id: Option<String>,
    /// The identifier for business profile
    #[schema(value_type = Option<String>)]
    pub profile_id: Option<common_utils::id_type::ProfileId>,
    /// Limit on the number of objects to return
    pub limit: Option<i64>,
    /// The starting point within a list of objects
    pub offset: Option<i64>,
    /// The time range for which objects are needed. TimeRange has two fields start_time and end_time from which objects can be filtered as per required scenarios (created_at, time less than, greater than etc)
    #[serde(flatten)]
    pub time_range: Option<TimeRange>,
    /// The amount to filter reufnds list. Amount takes two option fields start_amount and end_amount from which objects can be filtered as per required scenarios (less_than, greater_than, equal_to and range)
    pub amount_filter: Option<AmountFilter>,
    /// The list of connectors to filter refunds list
    pub connector: Option<Vec<String>>,
    /// The list of merchant connector ids to filter the refunds list for selected label
    #[schema(value_type = Option<Vec<String>>)]
    pub merchant_connector_id: Option<Vec<common_utils::id_type::MerchantConnectorAccountId>>,
    /// The list of currencies to filter refunds list
    #[schema(value_type = Option<Vec<Currency>>)]
    pub currency: Option<Vec<enums::Currency>>,
    /// The list of refund statuses to filter refunds list
    #[schema(value_type = Option<Vec<RefundStatus>>)]
    pub refund_status: Option<Vec<enums::RefundStatus>>,
}
#[cfg(feature = "v2")]
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct RefundListRequest {
    /// The identifier for the payment
    #[schema(value_type = Option<String>)]
    pub payment_id: Option<common_utils::id_type::GlobalPaymentId>,
    /// The identifier for the refund
    #[schema(value_type = String)]
    pub refund_id: Option<common_utils::id_type::GlobalRefundId>,
    /// Limit on the number of objects to return
    pub limit: Option<i64>,
    /// The starting point within a list of objects
    pub offset: Option<i64>,
    /// The time range for which objects are needed. TimeRange has two fields start_time and end_time from which objects can be filtered as per required scenarios (created_at, time less than, greater than etc)
    #[serde(flatten)]
    pub time_range: Option<TimeRange>,
    /// The amount to filter reufnds list. Amount takes two option fields start_amount and end_amount from which objects can be filtered as per required scenarios (less_than, greater_than, equal_to and range)
    pub amount_filter: Option<AmountFilter>,
    /// The list of connectors to filter refunds list
    pub connector: Option<Vec<String>>,
    /// The list of merchant connector ids to filter the refunds list for selected label
    #[schema(value_type = Option<Vec<String>>)]
    pub connector_id_list: Option<Vec<common_utils::id_type::MerchantConnectorAccountId>>,
    /// The list of currencies to filter refunds list
    #[schema(value_type = Option<Vec<Currency>>)]
    pub currency: Option<Vec<enums::Currency>>,
    /// The list of refund statuses to filter refunds list
    #[schema(value_type = Option<Vec<RefundStatus>>)]
    pub refund_status: Option<Vec<enums::RefundStatus>>,
}
#[derive(Debug, Clone, Eq, PartialEq, Serialize, ToSchema)]
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
    pub refund_status: Vec<enums::RefundStatus>,
}

#[derive(Clone, Debug, serde::Serialize, ToSchema)]
pub struct RefundListFilters {
    /// The map of available connector filters, where the key is the connector name and the value is a list of MerchantConnectorInfo instances
    pub connector: HashMap<String, Vec<MerchantConnectorInfo>>,
    /// The list of available currency filters
    #[schema(value_type = Vec<Currency>)]
    pub currency: Vec<enums::Currency>,
    /// The list of available refund status filters
    #[schema(value_type = Vec<RefundStatus>)]
    pub refund_status: Vec<enums::RefundStatus>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct RefundAggregateResponse {
    /// The list of refund status with their count
    pub status_with_count: HashMap<enums::RefundStatus, i64>,
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
    SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
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

impl From<RefundStatus> for enums::RefundStatus {
    fn from(status: RefundStatus) -> Self {
        match status {
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Review => Self::ManualReview,
            RefundStatus::Pending => Self::Pending,
            RefundStatus::Succeeded => Self::Success,
        }
    }
}
