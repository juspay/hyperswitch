use std::collections::HashMap;

use common_utils::types::{StringMinorUnit, TimeRange};
use masking::{Deserialize, Serialize};
use serde::de::Error;
use smithy::SmithyModel;
use time::PrimitiveDateTime;
use utoipa::ToSchema;

use super::enums::{Currency, DisputeStage, DisputeStatus};
use crate::{admin::MerchantConnectorInfo, files};

#[derive(Clone, Debug, Serialize, ToSchema, Eq, PartialEq)]
pub struct DisputeResponse {
    /// The identifier for dispute
    pub dispute_id: String,
    /// The identifier for payment_intent
    #[schema(value_type = String)]
    pub payment_id: common_utils::id_type::PaymentId,
    /// The identifier for payment_attempt
    pub attempt_id: String,
    /// The dispute amount
    pub amount: StringMinorUnit,
    /// The three-letter ISO currency code
    #[schema(value_type = Currency)]
    pub currency: Currency,
    /// Stage of the dispute
    pub dispute_stage: DisputeStage,
    /// Status of the dispute
    pub dispute_status: DisputeStatus,
    /// connector to which dispute is associated with
    pub connector: String,
    /// Status of the dispute sent by connector
    pub connector_status: String,
    /// Dispute id sent by connector
    pub connector_dispute_id: String,
    /// Reason of dispute sent by connector
    pub connector_reason: Option<String>,
    /// Reason code of dispute sent by connector
    pub connector_reason_code: Option<String>,
    /// Evidence deadline of dispute sent by connector
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub challenge_required_by: Option<PrimitiveDateTime>,
    /// Dispute created time sent by connector
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub connector_created_at: Option<PrimitiveDateTime>,
    /// Dispute updated time sent by connector
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub connector_updated_at: Option<PrimitiveDateTime>,
    /// Time at which dispute is received
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    /// The `profile_id` associated with the dispute
    #[schema(value_type = Option<String>)]
    pub profile_id: Option<common_utils::id_type::ProfileId>,
    /// The `merchant_connector_id` of the connector / processor through which the dispute was processed
    #[schema(value_type = Option<String>)]
    pub merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
}

#[derive(Clone, Debug, Serialize, ToSchema, Eq, PartialEq, SmithyModel)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct DisputeResponsePaymentsRetrieve {
    /// The identifier for dispute
    #[smithy(value_type = "String")]
    pub dispute_id: String,
    /// Stage of the dispute
    #[smithy(value_type = "DisputeStage")]
    pub dispute_stage: DisputeStage,
    /// Status of the dispute
    #[smithy(value_type = "DisputeStatus")]
    pub dispute_status: DisputeStatus,
    /// Status of the dispute sent by connector
    #[smithy(value_type = "String")]
    pub connector_status: String,
    /// Dispute id sent by connector
    #[smithy(value_type = "String")]
    pub connector_dispute_id: String,
    /// Reason of dispute sent by connector
    #[smithy(value_type = "Option<String>")]
    pub connector_reason: Option<String>,
    /// Reason code of dispute sent by connector
    #[smithy(value_type = "Option<String>")]
    pub connector_reason_code: Option<String>,
    /// Evidence deadline of dispute sent by connector
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    #[smithy(value_type = "Option<String>")]
    pub challenge_required_by: Option<PrimitiveDateTime>,
    /// Dispute created time sent by connector
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    #[smithy(value_type = "Option<String>")]
    pub connector_created_at: Option<PrimitiveDateTime>,
    /// Dispute updated time sent by connector
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    #[smithy(value_type = "Option<String>")]
    pub connector_updated_at: Option<PrimitiveDateTime>,
    /// Time at which dispute is received
    #[serde(with = "common_utils::custom_serde::iso8601")]
    #[smithy(value_type = "String")]
    pub created_at: PrimitiveDateTime,
}

#[derive(Debug, Serialize, Deserialize, strum::Display, Clone)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum EvidenceType {
    CancellationPolicy,
    CustomerCommunication,
    CustomerSignature,
    Receipt,
    RefundPolicy,
    ServiceDocumentation,
    ShippingDocumentation,
    InvoiceShowingDistinctTransactions,
    RecurringTransactionAgreement,
    UncategorizedFile,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct DisputeEvidenceBlock {
    /// Evidence type
    pub evidence_type: EvidenceType,
    /// File metadata
    pub file_metadata_response: files::FileMetadataResponse,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DisputeListGetConstraints {
    /// The identifier for dispute
    pub dispute_id: Option<String>,
    /// The payment_id against which dispute is raised
    pub payment_id: Option<common_utils::id_type::PaymentId>,
    /// Limit on the number of objects to return
    pub limit: Option<u32>,
    /// The starting point within a list of object
    pub offset: Option<u32>,
    /// The identifier for business profile
    #[schema(value_type = Option<String>)]
    pub profile_id: Option<common_utils::id_type::ProfileId>,
    /// The comma separated list of status of the disputes
    #[serde(default, deserialize_with = "parse_comma_separated")]
    pub dispute_status: Option<Vec<DisputeStatus>>,
    /// The comma separated list of stages of the disputes
    #[serde(default, deserialize_with = "parse_comma_separated")]
    pub dispute_stage: Option<Vec<DisputeStage>>,
    /// Reason for the dispute
    pub reason: Option<String>,
    /// The comma separated list of connectors linked to disputes
    #[serde(default, deserialize_with = "parse_comma_separated")]
    pub connector: Option<Vec<String>>,
    /// The comma separated list of currencies of the disputes
    #[serde(default, deserialize_with = "parse_comma_separated")]
    pub currency: Option<Vec<Currency>>,
    /// The merchant connector id to filter the disputes list
    pub merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    /// The time range for which objects are needed. TimeRange has two fields start_time and end_time from which objects can be filtered as per required scenarios (created_at, time less than, greater than etc).
    #[serde(flatten)]
    pub time_range: Option<TimeRange>,
}

#[derive(Clone, Debug, serde::Serialize, ToSchema)]
pub struct DisputeListFilters {
    /// The map of available connector filters, where the key is the connector name and the value is a list of MerchantConnectorInfo instances
    pub connector: HashMap<String, Vec<MerchantConnectorInfo>>,
    /// The list of available currency filters
    pub currency: Vec<Currency>,
    /// The list of available dispute status filters
    pub dispute_status: Vec<DisputeStatus>,
    /// The list of available dispute stage filters
    pub dispute_stage: Vec<DisputeStage>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SubmitEvidenceRequest {
    ///Dispute Id
    pub dispute_id: String,
    /// Logs showing the usage of service by customer
    pub access_activity_log: Option<String>,
    /// Billing address of the customer
    pub billing_address: Option<String>,
    /// File Id of cancellation policy
    pub cancellation_policy: Option<String>,
    /// Details of showing cancellation policy to customer before purchase
    pub cancellation_policy_disclosure: Option<String>,
    /// Details telling why customer's subscription was not cancelled
    pub cancellation_rebuttal: Option<String>,
    /// File Id of customer communication
    pub customer_communication: Option<String>,
    /// Customer email address
    pub customer_email_address: Option<String>,
    /// Customer name
    pub customer_name: Option<String>,
    /// IP address of the customer
    pub customer_purchase_ip: Option<String>,
    /// Fild Id of customer signature
    pub customer_signature: Option<String>,
    /// Product Description
    pub product_description: Option<String>,
    /// File Id of receipt
    pub receipt: Option<String>,
    /// File Id of refund policy
    pub refund_policy: Option<String>,
    /// Details of showing refund policy to customer before purchase
    pub refund_policy_disclosure: Option<String>,
    /// Details why customer is not entitled to refund
    pub refund_refusal_explanation: Option<String>,
    /// Customer service date
    pub service_date: Option<String>,
    /// File Id service documentation
    pub service_documentation: Option<String>,
    /// Shipping address of the customer
    pub shipping_address: Option<String>,
    /// Delivery service that shipped the product
    pub shipping_carrier: Option<String>,
    /// Shipping date
    pub shipping_date: Option<String>,
    /// File Id shipping documentation
    pub shipping_documentation: Option<String>,
    /// Tracking number of shipped product
    pub shipping_tracking_number: Option<String>,
    /// File Id showing two distinct transactions when customer claims a payment was charged twice
    pub invoice_showing_distinct_transactions: Option<String>,
    /// File Id of recurring transaction agreement
    pub recurring_transaction_agreement: Option<String>,
    /// Any additional supporting file
    pub uncategorized_file: Option<String>,
    /// Any additional evidence statements
    pub uncategorized_text: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct DeleteEvidenceRequest {
    /// Id of the dispute
    pub dispute_id: String,
    /// Evidence Type to be deleted
    pub evidence_type: EvidenceType,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DisputeRetrieveRequest {
    /// The identifier for dispute
    pub dispute_id: String,
    /// Decider to enable or disable the connector call for dispute retrieve request
    pub force_sync: Option<bool>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct DisputesAggregateResponse {
    /// Different status of disputes with their count
    pub status_with_count: HashMap<DisputeStatus, i64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DisputeRetrieveBody {
    /// Decider to enable or disable the connector call for dispute retrieve request
    pub force_sync: Option<bool>,
}

fn parse_comma_separated<'de, D, T>(v: D) -> Result<Option<Vec<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Debug + std::fmt::Display + std::error::Error,
{
    let output = Option::<&str>::deserialize(v)?;
    output
        .map(|s| {
            s.split(",")
                .map(|x| x.parse::<T>().map_err(D::Error::custom))
                .collect::<Result<_, _>>()
        })
        .transpose()
}
